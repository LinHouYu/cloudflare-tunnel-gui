use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Default)]
pub struct AppState {
    server_process: Arc<Mutex<Option<Child>>>,
    client_process: Arc<Mutex<Option<Child>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TunnelInfo {
    pub id: String,
    pub name: String,
    pub created: String,
    pub connections: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogPayload {
    pub message: String,
    pub level: String,
    pub source: String,
}

/// 优先获取本地应用目录下的 cloudflared 可执行文件，不存在时回退到系统环境变量 PATH 中的程序
pub fn get_cloudflared_executable() -> PathBuf {
    #[cfg(target_os = "windows")]
    let exe_name = "cloudflared.exe";
    #[cfg(not(target_os = "windows"))]
    let exe_name = "cloudflared";

    // 1. 检查当前工作目录 / 应用根目录
    if let Ok(cwd) = std::env::current_dir() {
        let local_path = cwd.join(exe_name);
        if local_path.exists() {
            return local_path;
        }
    }

    // 2. 检查主程序自身所在的目录
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let local_path = exe_dir.join(exe_name);
            if local_path.exists() {
                return local_path;
            }
        }
    }

    // 3. 回退到系统 PATH
    PathBuf::from(exe_name)
}

fn create_base_command() -> Command {
    let program = get_cloudflared_executable();
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[tauri::command]
fn list_tunnels() -> Result<Vec<TunnelInfo>, String> {
    let mut cmd = create_base_command();
    cmd.args(["tunnel", "list"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "未找到 cloudflared 可执行文件。请先在「杂项与关于」Tab 点击「安装 cloudflared」或将其放置在应用根目录下。".to_string()
        } else {
            format!("执行 cloudflared 失败: {}", e)
        }
    })?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut list = Vec::new();

    for line in stdout_str.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("You can obtain")
            || trimmed.starts_with("ID ")
            || trimmed.starts_with("----")
        {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 4 {
            let id = parts[0].to_string();
            let name = parts[1].to_string();
            let created = parts[2].to_string();
            let connections = parts[3..].join(" ");
            list.push(TunnelInfo {
                id,
                name,
                created,
                connections,
            });
        } else if parts.len() == 3 {
            list.push(TunnelInfo {
                id: parts[0].to_string(),
                name: parts[1].to_string(),
                created: parts[2].to_string(),
                connections: String::new(),
            });
        }
    }

    Ok(list)
}

#[tauri::command]
fn create_tunnel(name: String) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() || !trimmed.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err("隧道名不能为空且只能包含纯字母 (a-z, A-Z)".to_string());
    }

    let mut cmd = create_base_command();
    cmd.args(["tunnel", "create", trimmed])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd.output().map_err(|e| format!("执行命令失败: {}", e))?;
    let out_str = String::from_utf8_lossy(&output.stdout).to_string();
    let err_str = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(if out_str.trim().is_empty() { err_str } else { out_str })
    } else {
        Err(if !err_str.trim().is_empty() { err_str } else { out_str })
    }
}

#[tauri::command]
fn delete_tunnel(
    name: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("隧道 ID / 名称不能为空".to_string());
    }

    // ── 先 kill 本进程管理的 server 子进程，避免 1022 "active connections" ──
    // cloudflared 禁止删除有活跃连接的隧道，必须先让本进程的 child 退出。
    if let Ok(mut guard) = state.server_process.lock() {
        if let Some(ref mut child) = *guard {
            println!("[delete_tunnel] 检测到运行中的服务端子进程，正在 kill...");
            let _ = child.kill();
            let _ = child.wait(); // 回收 zombie，避免 SIGCHLD 残留
            *guard = None;
            println!("[delete_tunnel] 服务端子进程已终止");
        }
    }

    println!("[delete_tunnel] 执行: cloudflared tunnel delete -f {}", trimmed);

    // -f / --force 强制删除，即使 Cloudflare 远端仍有活跃连接记录也能成功
    let output = create_base_command()
        .args(["tunnel", "delete", "-f", trimmed])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("执行删除命令失败: {}", e))?;

    let out_str = String::from_utf8_lossy(&output.stdout).into_owned();
    let err_str = String::from_utf8_lossy(&output.stderr).into_owned();

    println!("[delete_tunnel] stdout: {}", out_str.trim());
    println!("[delete_tunnel] stderr: {}", err_str.trim());

    if output.status.success() {
        Ok(format!("隧道 [{}] 已成功删除", trimmed))
    } else {
        Err(if !err_str.trim().is_empty() { err_str } else { out_str })
    }
}

// ─── 内部辅助：UUID 提取 / 目录解析 / 统一子进程调用 ───────────────────────

/// 执行单条 cloudflared 子命令，同步等待并统一错误处理。
/// 成功时返回 `(stdout, stderr)`，失败时返回 Err(stderr ∪ stdout)。
fn run_cloudflared(args: &[&str]) -> Result<(String, String), String> {
    println!("[run_cloudflared] 执行: cloudflared {}", args.join(" "));

    let output = create_base_command()
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "未找到 cloudflared 可执行文件，请先在「杂项与关于」页面点击「安装 cloudflared」"
                    .to_string()
            } else {
                format!("启动 cloudflared 子进程失败: {}", e)
            }
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    println!("[run_cloudflared] stdout: {}", stdout.trim());
    println!("[run_cloudflared] stderr: {}", stderr.trim());
    println!("[run_cloudflared] exit code: {:?}", output.status.code());

    if output.status.success() {
        Ok((stdout, stderr))
    } else {
        Err(if !stderr.trim().is_empty() { stderr } else { stdout })
    }
}

/// 从 `cloudflared tunnel create` 的输出或凭证目录中提取隧道 UUID。
///
/// ## 提取策略（按可靠性降序）
///
/// ### 策略 1 — 读取 JSON 凭证文件（最可靠）
/// cloudflared 创建成功后，总会在 `~/.cloudflared/<uuid>.json` 写入凭证。
/// 文件内容形如：`{"AccountTag":"...","TunnelSecret":"...","TunnelID":"<uuid>",...}`
/// 直接解析其中的 `TunnelID` 字段，不受输出格式/ANSI 颜色转义影响。
///
/// ### 策略 2 — 正则匹配纯文本输出（兜底）
/// 去除 ANSI 转义序列后，在 stderr/stdout 中匹配 UUID 格式字符串。
fn extract_tunnel_uuid(stdout: &str, stderr: &str, tunnel_name: &str) -> Result<String, String> {
    // ── 策略 1：从凭证 JSON 文件读取 TunnelID（最可靠，不受输出格式影响） ──
    let home = dirs::home_dir()
        .ok_or_else(|| "无法确定用户主目录（$HOME 未设置）".to_string())?;
    let config_dir = home.join(".cloudflared");

    println!("[extract_uuid] 扫描凭证目录: {}", config_dir.display());

    // 找到所有 <uuid>.json，解析其中包含当前 tunnel_name 的文件
    let uuid_re = uuid_regex();
    let mut best: Option<(std::time::SystemTime, String)> = None;

    if let Ok(entries) = fs::read_dir(&config_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            if !uuid_re.is_match(&stem) {
                continue; // 跳过 cert.pem、config.yml 等
            }

            // 读取 JSON 确认 TunnelName 匹配（避免拿到旧隧道的 UUID）
            if let Ok(content) = fs::read_to_string(&path) {
                let name_matches = content.contains(&format!("\"TunnelName\":\"{}\"", tunnel_name))
                    || content.contains(&format!("\"Name\":\"{}\"", tunnel_name));
                let uuid_in_file: Option<String> = {
                    // 直接从 JSON 文本中提取 TunnelID（避免引入 serde_json 额外解析）
                    extract_json_string_field(&content, "TunnelID")
                        .or_else(|| extract_json_string_field(&content, "TunnelId"))
                };

                if let Some(ref uuid) = uuid_in_file {
                    println!(
                        "[extract_uuid] 找到凭证文件: {} → TunnelID: {}, 名称匹配: {}",
                        path.display(),
                        uuid,
                        name_matches
                    );
                    if name_matches {
                        // 名称精确匹配，直接返回
                        println!("[extract_uuid] ✅ 策略 1（精确匹配）UUID: {}", uuid);
                        return Ok(uuid.clone());
                    }
                    // 不匹配名称，但作为时间最新者备用
                    if let Ok(mtime) = entry.metadata().and_then(|m| m.modified()) {
                        let is_newer = best.as_ref().map_or(true, |(t, _)| mtime > *t);
                        if is_newer {
                            best = Some((mtime, uuid.clone()));
                        }
                    }
                }
            }
        }
    }

    // 名称精确匹配未找到，使用最新创建的 UUID（适合刚创建的隧道）
    if let Some((_, uuid)) = best {
        println!("[extract_uuid] ✅ 策略 1（最新文件）UUID: {}", uuid);
        return Ok(uuid);
    }

    // ── 策略 2：从 cloudflared 的输出文本中正则提取（兜底） ──
    // 先去除 ANSI 颜色转义序列，再做匹配
    let clean_stderr = strip_ansi(stderr);
    let clean_stdout = strip_ansi(stdout);

    println!("[extract_uuid] 策略 1 未命中，尝试正则匹配...");
    println!("[extract_uuid] 去色后 stderr: {}", clean_stderr.trim());
    println!("[extract_uuid] 去色后 stdout: {}", clean_stdout.trim());

    for line in clean_stderr.lines().chain(clean_stdout.lines()) {
        if let Some(m) = uuid_re.find(line) {
            let uuid = m.as_str().to_string();
            println!("[extract_uuid] ✅ 策略 2（正则）UUID: {}", uuid);
            return Ok(uuid);
        }
    }

    // 两种策略都失败
    Err(format!(
        "无法从 cloudflared 输出中提取隧道 UUID。\n\
         原始 stdout:\n{}\n\
         原始 stderr:\n{}\n\
         请检查 ~/.cloudflared/ 目录下是否存在 <uuid>.json 凭证文件。",
        stdout.trim(),
        stderr.trim()
    ))
}

/// 从 JSON 文本（不依赖 serde_json）中提取指定字段的字符串值。
/// 仅适用于简单的 `"Key":"Value"` 格式。
fn extract_json_string_field(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let end = json[start..].find('"')? + start;
    Some(json[start..end].to_string())
}

/// 惰性初始化 UUID 正则（编译一次，全局复用）。
fn uuid_regex() -> &'static regex::Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
        )
        .expect("UUID regex is valid")
    })
}

/// 去除字符串中的 ANSI 终端颜色/样式转义序列。
/// cloudflared 在终端模式下会在输出中混入这些序列，导致正则匹配失败。
fn strip_ansi(s: &str) -> String {
    // ANSI escape: ESC [ ... m  （最常见的 SGR 序列）
    use std::sync::OnceLock;
    static ANSI_RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = ANSI_RE.get_or_init(|| {
        regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").expect("ANSI regex is valid")
    });
    re.replace_all(s, "").into_owned()
}

/// **两阶段原子化**创建隧道并强制绑定 DNS CNAME：
///
/// - 阶段 1：`tunnel create <tunnel_name>` → 从 JSON 凭证文件提取 UUID（最可靠）
/// - 阶段 2：`tunnel route dns -f <uuid> <domain>`
///
/// `-f` / `--overwrite-dns` 强制覆盖已有 CNAME，
/// 彻底规避遗留死链与 DNS 记录冲突，无需用户进入 Cloudflare Dashboard。
#[tauri::command]
fn create_and_route_tunnel(tunnel_name: String, domain: String) -> Result<String, String> {
    let name = tunnel_name.trim().to_string();
    let domain = domain.trim().to_string();

    if name.is_empty() {
        return Err("隧道名不能为空".to_string());
    }
    if domain.is_empty() {
        return Err("域名不能为空".to_string());
    }

    println!("[create_and_route_tunnel] 开始：name={}, domain={}", name, domain);

    // ── 阶段 1：创建隧道 ──────────────────────────────────────────────────────
    let (stdout, stderr) = run_cloudflared(&["tunnel", "create", &name])
        .map_err(|e| format!("[阶段 1 / 创建隧道] 执行失败:\n{}", e))?;

    println!("[create_and_route_tunnel] 阶段 1 完成，开始提取 UUID...");

    let uuid = extract_tunnel_uuid(&stdout, &stderr, &name)
        .map_err(|e| format!("[阶段 1 / UUID 提取] {}", e))?;

    println!("[create_and_route_tunnel] ✅ 获取到 UUID: {}", uuid);

    // ── 阶段 2：强制绑定 DNS CNAME（-f 强制覆盖） ────────────────────────────
    println!(
        "[create_and_route_tunnel] 阶段 2：route dns -f {} {}",
        uuid, domain
    );

    run_cloudflared(&["tunnel", "route", "dns", "-f", &uuid, &domain])
        .map_err(|e| format!("[阶段 2 / DNS 绑定] 失败 (UUID: {}):\n{}", uuid, e))?;

    println!("[create_and_route_tunnel] ✅ DNS 绑定成功");

    Ok(format!(
        "✅ 隧道 [{name}] 已创建并绑定 DNS！\n   UUID: {uuid}\n   CNAME 已强制覆盖写入: {domain}"
    ))
}

#[tauri::command]
fn start_server_tunnel(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    port: String,
) -> Result<String, String> {
    let name_trimmed = name.trim();
    let port_trimmed = port.trim();

    if name_trimmed.is_empty() || !name_trimmed.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err("隧道名字必须为纯字母".to_string());
    }
    if port_trimmed.is_empty() || port_trimmed.parse::<u16>().is_err() {
        return Err("端口号必须为 1-65535 的纯数字".to_string());
    }

    let mut proc_guard = state.server_process.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut child) = *proc_guard {
        let _ = child.kill();
        *proc_guard = None;
    }

    let url_arg = format!("tcp://127.0.0.1:{}", port_trimmed);
    let mut cmd = create_base_command();
    cmd.args(["tunnel", "--name", name_trimmed, "--url", &url_arg])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "未找到 cloudflared 程序，请先点击「安装 cloudflared」".to_string()
        } else {
            format!("启动服务端隧道失败: {}", e)
        }
    })?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let app_clone1 = app.clone();
    if let Some(out) = stdout {
        thread::spawn(move || {
            let reader = BufReader::new(out);
            for line in reader.lines().flatten() {
                let _ = app_clone1.emit(
                    "log-message",
                    LogPayload {
                        message: line,
                        level: "info".to_string(),
                        source: "server".to_string(),
                    },
                );
            }
        });
    }

    let app_clone2 = app.clone();
    if let Some(err) = stderr {
        thread::spawn(move || {
            let reader = BufReader::new(err);
            for line in reader.lines().flatten() {
                let level = if line.contains("ERR") || line.contains("error") {
                    "error"
                } else if line.contains("WRN") || line.contains("warn") {
                    "warn"
                } else {
                    "info"
                };
                let _ = app_clone2.emit(
                    "log-message",
                    LogPayload {
                        message: line,
                        level: level.to_string(),
                        source: "server".to_string(),
                    },
                );
            }
        });
    }

    *proc_guard = Some(child);

    let start_msg = format!("已启动服务端隧道 [{}] 本地转发端口 [{}]", name_trimmed, port_trimmed);
    let _ = app.emit(
        "log-message",
        LogPayload {
            message: format!("[INFO] {}", start_msg),
            level: "success".to_string(),
            source: "server".to_string(),
        },
    );

    Ok(start_msg)
}

#[tauri::command]
fn start_temp_tunnel(
    app: AppHandle,
    state: State<'_, AppState>,
    port: String,
) -> Result<String, String> {
    let port_trimmed = port.trim();

    if port_trimmed.is_empty() || port_trimmed.parse::<u16>().is_err() {
        return Err("端口号必须为 1-65535 的纯数字".to_string());
    }

    let mut proc_guard = state.server_process.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut child) = *proc_guard {
        let _ = child.kill();
        let _ = child.wait();
        *proc_guard = None;
    }

    let url_arg = format!("tcp://localhost:{}", port_trimmed);
    let mut cmd = create_base_command();
    cmd.args(["tunnel", "--url", &url_arg])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "未找到 cloudflared 程序，请先点击「安装 cloudflared」".to_string()
        } else {
            format!("启动临时隧道失败: {}", e)
        }
    })?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let app_clone1 = app.clone();
    if let Some(out) = stdout {
        thread::spawn(move || {
            let reader = BufReader::new(out);
            for line in reader.lines().flatten() {
                let _ = app_clone1.emit(
                    "log-message",
                    LogPayload {
                        message: line,
                        level: "info".to_string(),
                        source: "server".to_string(),
                    },
                );
            }
        });
    }

    let app_clone2 = app.clone();
    if let Some(err) = stderr {
        thread::spawn(move || {
            let reader = BufReader::new(err);
            for line in reader.lines().flatten() {
                let level = if line.contains("ERR") || line.contains("error") {
                    "error"
                } else if line.contains("WRN") || line.contains("warn") {
                    "warn"
                } else {
                    "info"
                };
                let _ = app_clone2.emit(
                    "log-message",
                    LogPayload {
                        message: line,
                        level: level.to_string(),
                        source: "server".to_string(),
                    },
                );
            }
        });
    }

    *proc_guard = Some(child);

    let start_msg = format!("已启动免配置临时隧道 (转发端口: {})，正在连接 Cloudflare 获取临时域名...", port_trimmed);
    let _ = app.emit(
        "log-message",
        LogPayload {
            message: format!("[INFO] {}", start_msg),
            level: "success".to_string(),
            source: "server".to_string(),
        },
    );

    Ok(start_msg)
}

#[tauri::command]
fn stop_server_tunnel(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let mut proc_guard = state.server_process.lock().map_err(|e| e.to_string())?;
    if let Some(mut child) = proc_guard.take() {
        let _ = child.kill();
        let msg = "[INFO] 服务端隧道已停止".to_string();
        let _ = app.emit(
            "log-message",
            LogPayload {
                message: msg.clone(),
                level: "warn".to_string(),
                source: "server".to_string(),
            },
        );
        Ok("服务端隧道已停止".to_string())
    } else {
        Ok("当前没有正在运行的服务端隧道".to_string())
    }
}

#[tauri::command]
fn start_client_tunnel(
    app: AppHandle,
    state: State<'_, AppState>,
    domain: String,
    port: String,
) -> Result<String, String> {
    let domain_trimmed = domain.trim();
    let port_trimmed = port.trim();

    if domain_trimmed.is_empty() {
        return Err("隧道域名不能为空".to_string());
    }
    if port_trimmed.is_empty() || port_trimmed.parse::<u16>().is_err() {
        return Err("本地监听端口必须为 1-65535 的纯数字".to_string());
    }

    let mut proc_guard = state.client_process.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut child) = *proc_guard {
        let _ = child.kill();
        *proc_guard = None;
    }

    let url_arg = format!("tcp://127.0.0.1:{}", port_trimmed);
    let mut cmd = create_base_command();
    cmd.args(["access", "tcp", "--hostname", domain_trimmed, "--url", &url_arg])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "未找到 cloudflared 命令，请先点击「安装 cloudflared」".to_string()
        } else {
            format!("启动客户端隧道失败: {}", e)
        }
    })?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let app_clone1 = app.clone();
    if let Some(out) = stdout {
        thread::spawn(move || {
            let reader = BufReader::new(out);
            for line in reader.lines().flatten() {
                let _ = app_clone1.emit(
                    "log-message",
                    LogPayload {
                        message: line,
                        level: "info".to_string(),
                        source: "client".to_string(),
                    },
                );
            }
        });
    }

    let app_clone2 = app.clone();
    if let Some(err) = stderr {
        thread::spawn(move || {
            let reader = BufReader::new(err);
            for line in reader.lines().flatten() {
                let level = if line.contains("ERR") || line.contains("error") {
                    "error"
                } else if line.contains("WRN") || line.contains("warn") {
                    "warn"
                } else {
                    "info"
                };
                let _ = app_clone2.emit(
                    "log-message",
                    LogPayload {
                        message: line,
                        level: level.to_string(),
                        source: "client".to_string(),
                    },
                );
            }
        });
    }

    *proc_guard = Some(child);

    let start_msg = format!("已连接客户端隧道 [{}] -> 本地监听端口 [{}]", domain_trimmed, port_trimmed);
    let _ = app.emit(
        "log-message",
        LogPayload {
            message: format!("[INFO] {}", start_msg),
            level: "success".to_string(),
            source: "client".to_string(),
        },
    );

    Ok(start_msg)
}

#[tauri::command]
fn stop_client_tunnel(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let mut proc_guard = state.client_process.lock().map_err(|e| e.to_string())?;
    if let Some(mut child) = proc_guard.take() {
        let _ = child.kill();
        let msg = "[INFO] 客户端连接已断开".to_string();
        let _ = app.emit(
            "log-message",
            LogPayload {
                message: msg.clone(),
                level: "warn".to_string(),
                source: "client".to_string(),
            },
        );
        Ok("客户端连接已断开".to_string())
    } else {
        Ok("当前没有正在运行的客户端连接".to_string())
    }
}

#[tauri::command]
fn is_server_running(state: State<'_, AppState>) -> bool {
    if let Ok(mut guard) = state.server_process.lock() {
        if let Some(ref mut child) = *guard {
            match child.try_wait() {
                Ok(None) => true,
                _ => {
                    *guard = None;
                    false
                }
            }
        } else {
            false
        }
    } else {
        false
    }
}

#[tauri::command]
fn is_client_running(state: State<'_, AppState>) -> bool {
    if let Ok(mut guard) = state.client_process.lock() {
        if let Some(ref mut child) = *guard {
            match child.try_wait() {
                Ok(None) => true,
                _ => {
                    *guard = None;
                    false
                }
            }
        } else {
            false
        }
    } else {
        false
    }
}

#[tauri::command]
fn check_cloudflared_version() -> Result<String, String> {
    let exe_path = get_cloudflared_executable();
    let mut cmd = create_base_command();
    cmd.arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "未找到 cloudflared 可执行程序，请先点击「安装 cloudflared」".to_string()
        } else {
            format!("检查版本失败: {}", e)
        }
    })?;

    let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let err = String::from_utf8_lossy(&output.stderr).trim().to_string();

    let version_str = if !out.is_empty() {
        out
    } else if !err.is_empty() {
        err
    } else {
        "未知版本".to_string()
    };

    Ok(format!("{} (路径: {})", version_str, exe_path.display()))
}

#[tauri::command]
fn update_cloudflared(app: AppHandle) -> Result<String, String> {
    // 检查项目目录下是否已有程序
    let exe_path = get_cloudflared_executable();
    let _ = app.emit(
        "log-message",
        LogPayload {
            message: format!("[INFO] 正在检查 cloudflared 更新 (当前目标路径: {})...", exe_path.display()),
            level: "info".to_string(),
            source: "misc".to_string(),
        },
    );

    // 优先调用内置 update 指令
    let mut cmd = create_base_command();
    cmd.arg("update")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd.output().map_err(|e| format!("执行更新检查失败: {}", e))?;
    let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let err = String::from_utf8_lossy(&output.stderr).trim().to_string();

    let res = if !out.is_empty() { out } else { err };
    let _ = app.emit(
        "log-message",
        LogPayload {
            message: format!("[INFO] 更新检查结果: {}", res),
            level: "info".to_string(),
            source: "misc".to_string(),
        },
    );

    Ok(res)
}

#[tauri::command]
fn download_and_install_cloudflared(
    app: AppHandle,
    download_url: String,
    filename: String,
) -> Result<String, String> {
    let app_c = app.clone();
    thread::spawn(move || {
        let _ = app_c.emit(
            "log-message",
            LogPayload {
                message: format!("[INFO] 开始连接官方源下载 cloudflared: {}", download_url),
                level: "info".to_string(),
                source: "misc".to_string(),
            },
        );

        let agent = ureq::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build();

        let response = match agent
            .get(&download_url)
            .set("User-Agent", "Cloudflare-Tunnel-GUI")
            .call()
        {
            Ok(res) => res,
            Err(err) => {
                let _ = app_c.emit(
                    "log-message",
                    LogPayload {
                        message: format!("[ERROR] 下载请求失败: {}", err),
                        level: "error".to_string(),
                        source: "misc".to_string(),
                    },
                );
                return;
            }
        };

        let total_size: u64 = response
            .header("content-length")
            .and_then(|len| len.parse().ok())
            .unwrap_or(0);

        let mut reader = response.into_reader();
        let target_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        #[cfg(target_os = "windows")]
        let target_exe_name = "cloudflared.exe";
        #[cfg(not(target_os = "windows"))]
        let target_exe_name = "cloudflared";

        let temp_file_path = target_dir.join(format!("{}.tmp", filename));
        let final_exe_path = target_dir.join(target_exe_name);

        let mut file = match File::create(&temp_file_path) {
            Ok(f) => f,
            Err(e) => {
                let _ = app_c.emit(
                    "log-message",
                    LogPayload {
                        message: format!("[ERROR] 创建临时下载文件失败: {}", e),
                        level: "error".to_string(),
                        source: "misc".to_string(),
                    },
                );
                return;
            }
        };

        let mut downloaded: u64 = 0;
        let mut buffer = [0u8; 65536]; // 64KB buffer
        let mut last_percent: u32 = 0;
        let mut last_log_time = std::time::Instant::now();

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    if let Err(e) = file.write_all(&buffer[..n]) {
                        let _ = app_c.emit(
                            "log-message",
                            LogPayload {
                                message: format!("[ERROR] 写入文件失败: {}", e),
                                level: "error".to_string(),
                                source: "misc".to_string(),
                            },
                        );
                        let _ = fs::remove_file(&temp_file_path);
                        return;
                    }
                    downloaded += n as u64;

                    if total_size > 0 {
                        let percent = ((downloaded as f64 / total_size as f64) * 100.0) as u32;
                        let now = std::time::Instant::now();
                        // 每提升 5% 或每隔 800ms 打印一次下载进度
                        if percent >= last_percent + 5 || now.duration_since(last_log_time).as_millis() > 800 {
                            last_percent = percent;
                            last_log_time = now;
                            let dl_mb = downloaded as f64 / 1024.0 / 1024.0;
                            let total_mb = total_size as f64 / 1024.0 / 1024.0;
                            let _ = app_c.emit(
                                "log-message",
                                LogPayload {
                                    message: format!(
                                        "[INFO] [下载进度] {:>3}% ({:.2} MB / {:.2} MB) 正在下载 {}...",
                                        percent, dl_mb, total_mb, filename
                                    ),
                                    level: "info".to_string(),
                                    source: "misc".to_string(),
                                },
                            );
                        }
                    }
                }
                Err(e) => {
                    let _ = app_c.emit(
                        "log-message",
                        LogPayload {
                            message: format!("[ERROR] 读取下载数据流中断: {}", e),
                            level: "error".to_string(),
                            source: "misc".to_string(),
                        },
                    );
                    let _ = fs::remove_file(&temp_file_path);
                    return;
                }
            }
        }

        drop(file);

        // 重命名临时文件为最终可执行文件
        if let Err(_e) = fs::rename(&temp_file_path, &final_exe_path) {
            // 如果存在 Windows 文件锁，尝试覆盖拷贝
            if let Err(copy_err) = fs::copy(&temp_file_path, &final_exe_path) {
                let _ = app_c.emit(
                    "log-message",
                    LogPayload {
                        message: format!("[ERROR] 安装替换 cloudflared 文件失败: {}", copy_err),
                        level: "error".to_string(),
                        source: "misc".to_string(),
                    },
                );
                let _ = fs::remove_file(&temp_file_path);
                return;
            }
            let _ = fs::remove_file(&temp_file_path);
        }

        // Unix 系统添加执行权限
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&final_exe_path, fs::Permissions::from_mode(0o755));
        }

        let _ = app_c.emit(
            "log-message",
            LogPayload {
                message: format!("[SUCCESS] [下载进度 100%] cloudflared 安装成功！程序已保存至应用根目录: {}", final_exe_path.display()),
                level: "success".to_string(),
                source: "misc".to_string(),
            },
        );

        // 验证运行安装后的版本
        if let Ok(ver_res) = check_cloudflared_version() {
            let _ = app_c.emit(
                "log-message",
                LogPayload {
                    message: format!("[INFO] 已成功载入运行版本: {}", ver_res),
                    level: "info".to_string(),
                    source: "misc".to_string(),
                },
            );
        }
    });

    Ok("已在后台开始下载与安装流程，请观察控制台实时进度".to_string())
}

#[tauri::command]
fn login_cloudflared(app: AppHandle) -> Result<String, String> {
    let mut cmd = create_base_command();
    cmd.args(["tunnel", "login"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "未找到 cloudflared 可执行程序，请先点击「安装 cloudflared」".to_string()
        } else {
            format!("执行授权登录失败: {}", e)
        }
    })?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let _ = app.emit(
        "log-message",
        LogPayload {
            message: "[INFO] 已启动 Cloudflared 授权，正在获取登录链接...".to_string(),
            level: "info".to_string(),
            source: "misc".to_string(),
        },
    );

    let app_c1 = app.clone();
    if let Some(out) = stdout {
        thread::spawn(move || {
            let reader = BufReader::new(out);
            let mut opened = false;
            for line in reader.lines().flatten() {
                let _ = app_c1.emit(
                    "log-message",
                    LogPayload {
                        message: line.clone(),
                        level: "info".to_string(),
                        source: "misc".to_string(),
                    },
                );
                if line.contains("https://") && !opened {
                    opened = true;
                    if let Some(idx) = line.find("https://") {
                        let url: String = line[idx..].split_whitespace().next().unwrap_or("").to_string();
                        if !url.is_empty() {
                            let _ = open::that(&url);
                            let _ = app_c1.emit(
                                "log-message",
                                LogPayload {
                                    message: format!("[INFO] 已自动在默认浏览器中打开授权链接: {}", url),
                                    level: "success".to_string(),
                                    source: "misc".to_string(),
                                },
                            );
                        }
                    }
                }
            }
        });
    }

    let app_c2 = app.clone();
    if let Some(err) = stderr {
        thread::spawn(move || {
            let reader = BufReader::new(err);
            let mut opened = false;
            for line in reader.lines().flatten() {
                let _ = app_c2.emit(
                    "log-message",
                    LogPayload {
                        message: line.clone(),
                        level: "info".to_string(),
                        source: "misc".to_string(),
                    },
                );
                if line.contains("https://") && !opened {
                    opened = true;
                    if let Some(idx) = line.find("https://") {
                        let url: String = line[idx..].split_whitespace().next().unwrap_or("").to_string();
                        if !url.is_empty() {
                            let _ = open::that(&url);
                            let _ = app_c2.emit(
                                "log-message",
                                LogPayload {
                                    message: format!("[INFO] 已自动在默认浏览器中打开授权链接: {}", url),
                                    level: "success".to_string(),
                                    source: "misc".to_string(),
                                },
                            );
                        }
                    }
                }
            }
        });
    }

    Ok("Cloudflared 授权流程已启动".to_string())
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("无法打开链接: {}", e))
}

#[tauri::command]
fn show_main_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}


#[tauri::command]
fn minimize_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_maximize_window(window: tauri::WebviewWindow) -> Result<bool, String> {
    let is_max = window.is_maximized().unwrap_or(false);
    if is_max {
        window.unmaximize().map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        window.maximize().map_err(|e| e.to_string())?;
        Ok(true)
    }
}

#[tauri::command]
fn close_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
fn is_window_maximized(window: tauri::WebviewWindow) -> bool {
    window.is_maximized().unwrap_or(false)
}

#[tauri::command]
fn exit_app(app: AppHandle, state: State<'_, AppState>) {
    if let Ok(mut guard) = state.server_process.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
        }
    }
    if let Ok(mut guard) = state.client_process.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
        }
    }
    app.exit(0);
}

#[tauri::command]
fn open_cloudflared_config_dir(app: AppHandle) -> Result<String, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .map_err(|_| "无法获取用户主目录".to_string())?;

    let config_dir = home.join(".cloudflared");

    if !config_dir.exists() {
        let _ = fs::create_dir_all(&config_dir);
    }

    let dir_str = config_dir.to_string_lossy().to_string();

    open::that(&config_dir).map_err(|e| format!("打开配置目录失败: {}", e))?;

    let _ = app.emit(
        "log-message",
        LogPayload {
            message: format!("[INFO] 已在文件资源管理器中打开配置目录: {}", dir_str),
            level: "info".to_string(),
            source: "misc".to_string(),
        },
    );

    Ok(dir_str)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            // 创建系统托盘右键菜单
            let show_i = MenuItem::with_id(app, "show", "🖥️ 显示主窗口", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "🚪 退出程序", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let mut builder = TrayIconBuilder::with_id("main-tray")
                .tooltip("Cloudflare Tunnel GUI")
                .menu(&menu)
                .show_menu_on_left_click(false);

            if let Some(icon) = app.default_window_icon() {
                builder = builder.icon(icon.clone());
            }

            builder
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                            let _ = window.emit("show-exit-confirm", ());
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                    | TrayIconEvent::DoubleClick {
                        button: MouseButton::Left,
                        ..
                    } => {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_tunnels,
            create_tunnel,
            create_and_route_tunnel,
            delete_tunnel,
            start_server_tunnel,
            start_temp_tunnel,
            stop_server_tunnel,
            start_client_tunnel,
            stop_client_tunnel,
            is_server_running,
            is_client_running,
            check_cloudflared_version,
            update_cloudflared,
            download_and_install_cloudflared,
            login_cloudflared,
            open_external_url,
            open_cloudflared_config_dir,
            show_main_window,
            exit_app,
            minimize_window,
            toggle_maximize_window,
            close_window,
            is_window_maximized
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
