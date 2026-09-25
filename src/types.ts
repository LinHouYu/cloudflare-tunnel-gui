export interface TunnelInfo {
  id: string;
  name: string;
  created: string;
  connections: string;
}

export interface LogEntry {
  id: string;
  timestamp: string;
  message: string;
  level: 'info' | 'warn' | 'error' | 'success';
  source: 'server' | 'client' | 'misc' | 'system';
}

export type LangKey = 'zh_CN' | 'zh_TW' | 'pt_BR' | 'es_ES' | 'en_US' | 'ja_JP';

export interface LangPack {
  title: string;
  header: string;
  lang_button: string;
  tabs: {
    server: string;
    client: string;
    misc: string;
  };
  server_tab: {
    mode_fixed: string;
    mode_temp: string;
    tunnel_name: string;
    port: string;
    tunnel_name_placeholder: string;
    port_placeholder: string;
    btn_create: string;
    btn_start: string;
    btn_stop: string;
    btn_get_temp_domain: string;
    btn_refresh: string;
    btn_delete: string;
    btn_clear_log: string;
    status_running: string;
    status_stopped: string;
    status_temp_ready: string;
    node_label: string;
    nodes: Record<string, string>;
    temp_mode_tip: string;
    temp_domain_title: string;
    temp_tag_active: string;
    temp_tag_applying: string;
    temp_copy_btn: string;
    temp_copy_success: string;
    temp_applying_text: string;
    temp_client_tip: string;
    list_title: string;
    list_subtitle: string;
    headers: {
      id: string;
      name: string;
      created: string;
      connections: string;
    };
    errors: {
      tunnel_invalid: string;
      port_invalid: string;
      no_selection: string;
      delete_confirm_title: string;
      delete_confirm_msg: string;
    };
  };
  client_tab: {
    domain: string;
    port: string;
    domain_placeholder: string;
    port_placeholder: string;
    btn_connect: string;
    btn_disconnect: string;
    status_connected: string;
    status_disconnected: string;
    errors: {
      domain_invalid: string;
      port_invalid: string;
    };
  };
  misc_tab: {
    btn_install: string;
    btn_install_desc: string;
    btn_open_config_dir: string;
    btn_open_config_dir_desc: string;
    config_dir_warning: string;
    btn_login: string;
    btn_check_version: string;
    btn_update: string;
    btn_download: string;
    donate_title: string;
    wechat_donate: string;
    usdt_donate: string;
    usdt_address: string;
    copy_address: string;
    copy_success: string;
    about_title: string;
    github_label: string;
    bilibili_label: string;
    author: string;
  };
  exit_modal: {
    title: string;
    message: string;
    btn_confirm: string;
    btn_cancel: string;
  };
  console: {
    title: string;
    filter_all: string;
    btn_clear: string;
    btn_copy: string;
    copy_logs_success: string;
    empty: string;
  };
}
