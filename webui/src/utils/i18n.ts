import { computed } from 'vue'
import { useSettingsStore } from '../stores/settings'

export const messages = {
  zh: {
    common: {
      cancel: '取消',
      confirm: '确定',
      save: '保存',
      delete: '删除',
      edit: '编辑',
      add: '添加',
      loading: '加载中...',
      success: '成功',
      failed: '失败',
      default: '默认',
      enabled: '启用',
      disabled: '禁用',
      saved: '已保存',
    },
    config: {
      load_failed: '配置加载失败',
      empty_content: '配置为空，未写入',
      save_verify_failed: '保存后校验失败',
      backup_used: '已使用本地备份恢复配置',
      backup_failed: '本地备份不可用，配置加载失败',
    },
    nav: {
      home: '主页',
      templates: '模板',
      apps: '应用',
      settings: '设置',
    },
    status: {
      title: '主页',
      items: {
        module_status: '模块信息',
        module_version: '模块版本',
        impersonated_apps_count: '伪装应用数量',
        templates_count: '机型模板数量',
        work_mode: '工作模式',
      },
      follow: {
        title: '关注模块',
        action: '获取项目动态',
        channels: '加入社群、联系作者并给仓库点个 Star',
        dialog_title: '关注模块',
        module_name: '模块名称',
        author: '作者',
        communities: '社群',
        repository: 'GitHub 仓库',
        telegram: 'Telegram 群',
        qq_group: 'QQ群',
        open_link: '打开链接',
        repository_action_primary: '前往GitHub',
        repository_action_secondary: '给项目点个 Star',
      },
      mode: {
        lite: '轻量模式',
        full: '完整模式',
      },
    },
    templates: {
      title: '机型模板',
      actions: {
        online: '在线模板',
        new: '新建模板',
        add: '添加',
      },
      fields: {
        name: '模板名称',
        manufacturer: 'Manufacturer',
        brand: 'Brand',
        model: 'Model',
        device: 'Device',
        product: 'Product',
        name_field: 'Name (可选，仅 full 模式)',
        market_name: 'Market Name (可选，仅 full 模式)',
        fingerprint: 'Fingerprint',
        android_version: 'Android 版本',
        sdk_int: 'SDK 版本',
        characteristics: 'Characteristics (可选，仅 full 模式)',
        force_denylist_unmount: '强制卸载挂载点',
        mode: '工作模式 (可选)',
        packages: '应用包名列表 (可选)',
        system: '系统',
      },
      placeholders: {
        name: '例如：redmagic_9_pro',
        manufacturer: '例如：ZTE',
        brand: '例如：nubia',
        model: '例如：25010PN30C，NX769J',
        device: '例如：xuanyuan，NX769J',
        product: '例如：xuanyuan，NX769J',
        name_field: '例如：xuanyuan',
        market_name: '例如：REDMAGIC 9 Pro',
        fingerprint:
          '例如：nubia/NX769J/NX769J:14/UKQ1.230917.001/20240813.173312:user/release-keys',
        android_version: '例如：16, 15, 14',
        sdk_int: '例如：36, 35, 34',
        characteristics: '例如：tablet (仅 full 模式)',
        mode: '留空使用全局默认模式',
        packages: '输入或搜索应用包名',
      },
      options: {
        mode_lite: 'lite - 轻量模式（推荐，隐蔽性好）',
        mode_full: 'full - 完整模式（全面伪装，可能被检测）',
        mode_resetprop: 'resetprop - Resetprop 模式 (支持只读属性)',
      },
      empty: {
        title: '暂无机型模板',
        hint: '点击上方按钮创建新模板',
        packages: '暂无包名，可以输入包名或从已安装应用中选择',
      },
      dialog: {
        edit_title: '编辑模板',
        new_title: '新建模板',
        delete_title: '确认删除',
        delete_confirm: '确定要删除模板 "{name}" 吗？',
      },
      messages: {
        name_required: '请输入模板名称',
        saved: '模板已保存',
        deleted: '模板已删除',
        pkg_exists: '该包名已添加',
      },
      labels: {
        mode: '模式',
        packages: '应用包名',
        count_suffix: '个',
        version: '版本',
        version_code: '版本号',
        author: '作者',
        description: '描述',
      },
      values: {
        lite: 'lite (轻量)',
        full: 'full (完整)',
        resetprop: 'resetprop (Resetprop)',
      },
      search: {
        placeholder: '搜索模板名称、品牌或型号',
        no_results: '未找到匹配的模板',
      },
      categories: {
        all: '全部',
        common: '通用设备',
        gaming: '游戏设备',
        transcend: '破限设备',
      },
      online: {
        title: '在线模板库',
        subtitle: '先加载索引，再分批补齐全部模板详情，支持缓存和失败重试',
        loading: '正在加载在线模板...',
        retry: '重试',
        retry_failed: '重试失败项',
        retry_detail: '重试详情',
        brand: '品牌',
        model: '型号',
        market_name: '市场名',
        importing: '导入中...',
        import: '导入',
        back: '返回模板页',
        empty_category: '该分类暂无模板',
        empty_filtered: '当前筛选条件下没有匹配模板',
        empty_library: '模板库为空，请刷新后重试',
        close: '关闭',
        source_button: '模板源：{source}',
        refresh: '刷新列表',
        search_placeholder: '搜索模板名称、品牌或分类',
        clear_filters: '清空筛选',
        progress_label: '详情加载',
        progress_pending: '等待开始',
        progress_partial: '已完成 {resolved}/{total}，剩余 {pending} 项',
        progress_complete: '已完成 {count}/{total}，失败 {failed} 项',
        sources: {
          gitee: 'Gitee',
          github: 'GitHub',
        },
        status: {
          source: '数据源：{source}',
          fallback: '已切换到备用源',
          cached: '正在显示缓存',
          pending: '待加载',
          loading: '加载中',
          ready: '已就绪',
          failed: '失败',
        },
        toasts: {
          start_loading: '开始加载在线模板...',
          fetching_list: '正在获取模板列表...',
          got_templates: '获取到 {count} 个模板',
          no_templates_toast: '未找到任何模板，可能是网络问题或 API 限流',
          list_loaded: '模板列表已加载，正在后台下载详细内容...',
          content_loaded: '成功加载 {count} 个模板内容',
          content_failed: '{count} 个模板内容加载失败',
          load_failed: '加载失败: {error}',
        },
        errors: {
          no_templates: '未找到任何模板，请检查网络连接或稍后重试',
          load_failed: '加载失败',
          detail_failed: '详情加载失败',
          empty_content: '模板内容为空',
          import_failed: '导入失败',
        },
        messages: {
          import_success: '模板 "{name}" 导入成功',
          exists_confirm: '模板 "{name}" 已存在，是否覆盖？',
          exists_title: '确认导入',
          overwrite: '覆盖',
        },
      },
    },
    apps: {
      title: '应用管理',
      search_placeholder: '搜索应用或包名',
      actions: {
        show_system: '显示系统应用',
        hide_system: '隐藏系统应用',
      },
      tabs: {
        all: '全部',
        configured: '已配置',
        unconfigured: '未配置',
      },
      status: {
        configured: '已配置',
        unconfigured: '未配置',
        not_installed: '未安装',
      },
      empty: {
        search: '未找到匹配的应用',
        configured: '暂无已配置的应用',
        unconfigured: '所有应用都已配置',
        all: '暂无应用',
      },
      dialog: {
        config_title: '配置 {name}',
        mode_template: '模板配置',
        mode_custom: '自定义配置',
        mode_remove: '移除配置',
        select_template_placeholder: '选择机型模板',
        search_template_placeholder: '搜索模板名称、品牌或型号',
        search_no_result: '无匹配模板，请尝试其他关键词',
        no_templates: '暂无模板，请先创建或导入',
        remove_hint: '确定要移除该应用的伪装配置吗？',
        remove_custom_config: '移除自定义配置',
        remove_template_config: '移除模板配置',
      },
      messages: {
        select_template: '请选择模板',
        saved: '配置已保存',
        custom_config_removed: '自定义配置已移除',
        template_config_removed: '模板配置已移除',
      },
    },
    settings: {
      display: {
        title: '显示设置',
        theme: {
          label: '主题',
          desc: '选择界面主题',
          system: '跟随系统',
          light: '浅色模式',
          dark: '深色模式',
        },
        language: {
          label: '语言',
          desc: '选择界面语言',
          system: '跟随系统',
          zh: '简体中文',
          en: 'English',
        },
      },
      module: {
        title: '模块设置',
        default_mode: {
          label: '默认工作模式',
          desc: '选择模块的默认工作模式',
          lite: '轻量模式 (推荐)',
          full: '完整模式',
          resetprop: 'Resetprop 模式',
        },
        force_denylist_unmount: {
          label: '默认强制卸载挂载点',
          desc: '为目标应用启用 Zygisk 的 FORCE_DENYLIST_UNMOUNT',
        },
        debug: {
          label: '调试模式',
          desc: '启用后可在 logcat 中查看详细日志',
        },
      },
      tools: {
        title: '工具',
        convert: {
          label: '配置转换',
          desc: '将 system.prop 机型伪装模块转换为机型模板',
          btn: '开始转换',
        },
      },
      dialog: {
        result: {
          title: '转换结果',
          template_name_label: '模板名称',
          template_name_placeholder: '请输入模板名称',
          preview_label: '预览内容',
          btn_save: '保存为模板',
        },
      },
      messages: {
        default_mode_updated: '默认模式已更新',
        debug_enabled: '调试模式已启用',
        debug_disabled: '调试模式已关闭',
        read_failed: '读取文件失败或文件为空',
        convert_failed: '转换失败',
        save_failed: '保存失败',
        save_success: '保存成功',
        template_saved: '模板已保存',
      },
    },
  },
  en: {
    common: {
      cancel: 'Cancel',
      confirm: 'Confirm',
      save: 'Save',
      delete: 'Delete',
      edit: 'Edit',
      add: 'Add',
      loading: 'Loading...',
      success: 'Success',
      failed: 'Failed',
      default: 'Default',
      enabled: 'Enabled',
      disabled: 'Disabled',
      saved: 'Saved',
    },
    config: {
      load_failed: 'Failed to load configuration',
      empty_content: 'Configuration is empty, nothing written',
      save_verify_failed: 'Save verification failed',
      backup_used: 'Recovered configuration from local backup',
      backup_failed: 'Local backup unavailable, configuration load failed',
    },
    nav: {
      home: 'Home',
      templates: 'Templates',
      apps: 'Apps',
      settings: 'Settings',
    },
    status: {
      title: 'Home',
      items: {
        module_status: 'Module Info',
        module_version: 'Module Version',
        impersonated_apps_count: 'Impersonated Apps',
        templates_count: 'Templates Count',
        work_mode: 'Work Mode',
      },
      follow: {
        title: 'Follow Module',
        action: 'Get Project Updates',
        channels: 'Join communities, contact the author, and star the repository.',
        dialog_title: 'Follow Module',
        module_name: 'Module Name',
        author: 'Author',
        communities: 'Communities',
        repository: 'GitHub Repository',
        telegram: 'Telegram Group',
        qq_group: 'QQ Group',
        open_link: 'Open Link',
        repository_action_primary: 'Open GitHub',
        repository_action_secondary: 'Star the Project',
      },
      mode: {
        lite: 'Lite Mode',
        full: 'Full Mode',
      },
    },
    templates: {
      title: 'Device Templates',
      actions: {
        online: 'Online Templates',
        new: 'New Template',
        add: 'Add',
      },
      fields: {
        name: 'Template Name',
        manufacturer: 'Manufacturer',
        brand: 'Brand',
        model: 'Model',
        device: 'Device',
        product: 'Product',
        name_field: 'Name (Optional, full mode only)',
        market_name: 'Market Name (Optional, full mode only)',
        fingerprint: 'Fingerprint',
        android_version: 'Android Version',
        sdk_int: 'SDK Version',
        characteristics: 'Characteristics (Optional, full mode only)',
        force_denylist_unmount: 'Force Denylist Unmount',
        mode: 'Work Mode (Optional)',
        packages: 'Packages (Optional)',
        system: 'System',
      },
      placeholders: {
        name: 'e.g. redmagic_9_pro',
        manufacturer: 'e.g. ZTE',
        brand: 'e.g. nubia',
        model: 'e.g. 25010PN30C, NX769J',
        device: 'e.g. xuanyuan, NX769J',
        product: 'e.g. xuanyuan, NX769J',
        name_field: 'e.g. xuanyuan',
        market_name: 'e.g. REDMAGIC 9 Pro',
        fingerprint:
          'e.g. nubia/NX769J/NX769J:14/UKQ1.230917.001/20240813.173312:user/release-keys',
        android_version: 'e.g. 16, 15, 14',
        sdk_int: 'e.g. 36, 35, 34',
        characteristics: 'e.g. tablet (full mode only)',
        mode: 'Leave empty to use global default',
        packages: 'Enter or search package name',
      },
      options: {
        mode_lite: 'lite - Lite Mode\n(Recommended, Stealthy)',
        mode_full: 'full - Full Mode\n(Complete Spoofing, Detectable)',
        mode_resetprop: 'resetprop - Resetprop Mode (Supports read-only props)',
      },
      empty: {
        title: 'No Templates Found',
        hint: 'Click above button to create a new template',
        packages: 'No packages added. Enter manually or select from installed apps',
      },
      dialog: {
        edit_title: 'Edit Template',
        new_title: 'New Template',
        delete_title: 'Delete Confirmation',
        delete_confirm: 'Are you sure to delete template "{name}"?',
      },
      messages: {
        name_required: 'Please enter template name',
        saved: 'Template saved',
        deleted: 'Template deleted',
        pkg_exists: 'Package already added',
      },
      labels: {
        mode: 'Mode',
        packages: 'Packages',
        count_suffix: 'items',
        version: 'Version',
        version_code: 'Version Code',
        author: 'Author',
        description: 'Description',
      },
      values: {
        lite: 'lite (Lite)',
        full: 'full (Full)',
        resetprop: 'resetprop (Resetprop)',
      },
      search: {
        placeholder: 'Search by name, brand, or model',
        no_results: 'No matching templates found',
      },
      categories: {
        all: 'All',
        common: 'Common',
        gaming: 'Gaming',
        transcend: 'Transcend',
      },
      online: {
        title: 'Online Template Library',
        subtitle:
          'Load the catalog first, then stream all template details in the background with cache reuse and retry support.',
        loading: 'Loading online templates...',
        retry: 'Retry',
        retry_failed: 'Retry Failed',
        retry_detail: 'Retry Detail',
        brand: 'Brand',
        model: 'Model',
        market_name: 'Market Name',
        importing: 'Importing...',
        import: 'Import',
        back: 'Back to Templates',
        empty_category: 'No templates in this category',
        empty_filtered: 'No templates match the current filters',
        empty_library: 'The library is empty. Refresh and try again.',
        close: 'Close',
        source_button: 'Source: {source}',
        refresh: 'Refresh List',
        search_placeholder: 'Search by template name, brand, or category',
        clear_filters: 'Clear Filters',
        progress_label: 'Details Progress',
        progress_pending: 'Waiting to start',
        progress_partial: '{resolved}/{total} resolved, {pending} remaining',
        progress_complete: '{count}/{total} loaded, {failed} failed',
        sources: {
          gitee: 'Gitee',
          github: 'GitHub',
        },
        status: {
          source: 'Source: {source}',
          fallback: 'Using fallback source',
          cached: 'Showing cached data',
          pending: 'Pending',
          loading: 'Loading',
          ready: 'Ready',
          failed: 'Failed',
        },
        toasts: {
          start_loading: 'Starting load online templates...',
          fetching_list: 'Fetching template list...',
          got_templates: 'Got {count} templates',
          no_templates_toast: 'No templates found, possibly network issue or API rate limit',
          list_loaded: 'Template list loaded, downloading details in background...',
          content_loaded: 'Successfully loaded {count} template contents',
          content_failed: 'Failed to load {count} template contents',
          load_failed: 'Load failed: {error}',
        },
        errors: {
          no_templates: 'No templates found, please check network or try again later',
          load_failed: 'Load failed',
          detail_failed: 'Failed to load template detail',
          empty_content: 'Template content is empty',
          import_failed: 'Import failed',
        },
        messages: {
          import_success: 'Template "{name}" imported successfully',
          exists_confirm: 'Template "{name}" already exists, overwrite?',
          exists_title: 'Import Confirmation',
          overwrite: 'Overwrite',
        },
      },
    },
    apps: {
      title: 'App Management',
      search_placeholder: 'Search app or package',
      actions: {
        show_system: 'Show System Apps',
        hide_system: 'Hide System Apps',
      },
      tabs: {
        all: 'All',
        configured: 'Configured',
        unconfigured: 'Unconfigured',
      },
      status: {
        configured: 'Configured',
        unconfigured: 'Unconfigured',
        not_installed: 'Not installed',
      },
      empty: {
        search: 'No matching apps found',
        configured: 'No configured apps',
        unconfigured: 'All apps are configured',
        all: 'No apps found',
      },
      dialog: {
        config_title: 'Configure {name}',
        mode_template: 'Template Config',
        mode_custom: 'Custom Config',
        mode_remove: 'Remove Config',
        select_template_placeholder: 'Select Device Template',
        search_template_placeholder: 'Search by name, brand, or model',
        search_no_result: 'No matching templates, try another keyword',
        no_templates: 'No templates yet, create or import first',
        remove_hint: 'Are you sure to remove the spoofing config for this app?',
        remove_custom_config: 'Remove Custom Config',
        remove_template_config: 'Remove Template Config',
      },
      messages: {
        select_template: 'Please select a template',
        saved: 'Config saved',
        custom_config_removed: 'Custom config removed',
        template_config_removed: 'Template config removed',
      },
    },
    settings: {
      display: {
        title: 'Display Settings',
        theme: {
          label: 'Theme',
          desc: 'Choose interface theme',
          system: 'Follow System',
          light: 'Light Mode',
          dark: 'Dark Mode',
        },
        language: {
          label: 'Language',
          desc: 'Choose interface language',
          system: 'Follow System',
          zh: 'Simplified Chinese',
          en: 'English',
        },
      },
      module: {
        title: 'Module Settings',
        default_mode: {
          label: 'Default Work Mode',
          desc: 'Choose default work mode for module',
          lite: 'Lite Mode (Recommended)',
          full: 'Full Mode',
          resetprop: 'Resetprop Mode',
        },
        force_denylist_unmount: {
          label: 'Default Force Denylist Unmount',
          desc: 'Enable Zygisk FORCE_DENYLIST_UNMOUNT for target apps',
        },
        debug: {
          label: 'Debug Mode',
          desc: 'Enable to see detailed logs in logcat',
        },
      },
      tools: {
        title: 'Tools',
        convert: {
          label: 'Config Conversion',
          desc: 'Convert system.prop-based spoof module into a device template',
          btn: 'Start Conversion',
        },
      },
      dialog: {
        result: {
          title: 'Conversion Result',
          template_name_label: 'Template Name',
          template_name_placeholder: 'Please enter template name',
          preview_label: 'Preview Content',
          btn_save: 'Save as Template',
        },
      },
      messages: {
        default_mode_updated: 'Default mode updated',
        debug_enabled: 'Debug mode enabled',
        debug_disabled: 'Debug mode disabled',
        read_failed: 'Failed to read file or file is empty',
        convert_failed: 'Conversion failed',
        save_failed: 'Save failed',
        save_success: 'Saved successfully',
        template_saved: 'Template saved',
      },
    },
  },
}

export type Locale = 'zh' | 'en'

// Simple object path access
function getNestedValue(obj: unknown, path: string): string {
  return path.split('.').reduce((prev, curr) => {
    if (prev && typeof prev === 'object') {
      return (prev as Record<string, unknown>)[curr]
    }
    return null
  }, obj) as string
}

export function useI18n() {
  const settingsStore = useSettingsStore()

  const locale = computed(() => {
    if (settingsStore.language === 'system') {
      const sysLang = navigator.language
      return sysLang.startsWith('zh') ? 'zh' : 'en'
    }
    return settingsStore.language as Locale
  })

  function t(path: string, args?: Record<string, string | number>): string {
    const currentMessages = messages[locale.value]
    let value = getNestedValue(currentMessages, path)

    if (!value) {
      console.warn(`Translation missing for key: ${path}`)
      return path
    }

    if (args) {
      Object.entries(args).forEach(([key, val]) => {
        value = value.replace(`{${key}}`, String(val))
      })
    }

    return value
  }

  return {
    t,
    locale,
  }
}
