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
    nav: {
      status: '状态',
      templates: '模板',
      apps: '应用',
      settings: '设置',
    },
    status: {
      items: {
        module_status: '模块状态',
        module_version: '模块版本',
        impersonated_apps_count: '伪装应用数量',
        templates_count: '机型模板数量',
        work_mode: '工作模式',
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
        characteristics: 'Characteristics (可选，仅 full 模式)',
        force_denylist_unmount: '强制卸载挂载点',
        mode: '工作模式 (可选)',
        packages: '应用包名列表 (可选)',
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
      categories: {
        all: '全部',
        common: '通用设备',
        gaming: '游戏设备',
        transcend: '破限设备',
      },
      online: {
        title: '在线模板库',
        loading: '正在加载在线模板...',
        retry: '重试',
        brand: '品牌',
        model: '型号',
        market_name: '市场名',
        importing: '导入中...',
        import: '导入',
        empty_category: '该分类暂无模板',
        close: '关闭',
        refresh: '刷新列表',
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
        mode_template: '应用模板',
        mode_custom: '自定义配置',
        mode_remove: '移除配置',
        select_template_placeholder: '选择机型模板',
        search_template_placeholder: '搜索模板名称、品牌或型号',
        search_no_result: '无匹配模板，请尝试其他关键词',
        no_templates: '暂无模板，请先创建或导入',
        remove_hint: '确定要移除该应用的伪装配置吗？',
      },
      messages: {
        select_template: '请选择模板',
        saved: '配置已保存',
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
          desc: '将 system.prop 转换为机型模板',
          btn: '开始转换',
        },
      },
      dialog: {
        convert: {
          title: '配置转换',
          path_label: 'system.prop 文件路径',
          path_placeholder: '请输入 system.prop 文件的绝对路径',
          default_path_tip: '默认路径: /data/adb/device_faker/config/system.prop',
          btn_start: '开始转换',
        },
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
        input_path: '请输入文件路径',
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
    nav: {
      status: 'Status',
      templates: 'Templates',
      apps: 'Apps',
      settings: 'Settings',
    },
    status: {
      items: {
        module_status: 'Module Status',
        module_version: 'Module Version',
        impersonated_apps_count: 'Impersonated Apps',
        templates_count: 'Templates Count',
        work_mode: 'Work Mode',
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
        characteristics: 'Characteristics (Optional, full mode only)',
        force_denylist_unmount: 'Force Denylist Unmount',
        mode: 'Work Mode (Optional)',
        packages: 'Packages (Optional)',
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
      categories: {
        all: 'All',
        common: 'Common',
        gaming: 'Gaming',
        transcend: 'Transcend',
      },
      online: {
        title: 'Online Template Library',
        loading: 'Loading online templates...',
        retry: 'Retry',
        brand: 'Brand',
        model: 'Model',
        market_name: 'Market Name',
        importing: 'Importing...',
        import: 'Import',
        empty_category: 'No templates in this category',
        close: 'Close',
        refresh: 'Refresh List',
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
        mode_template: 'Apply Template',
        mode_custom: 'Custom Config',
        mode_remove: 'Remove Config',
        select_template_placeholder: 'Select Device Template',
        search_template_placeholder: 'Search by name, brand, or model',
        search_no_result: 'No matching templates, try another keyword',
        no_templates: 'No templates yet, create or import first',
        remove_hint: 'Are you sure to remove the spoofing config for this app?',
      },
      messages: {
        select_template: 'Please select a template',
        saved: 'Config saved',
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
          desc: 'Convert system.prop to device template',
          btn: 'Start Conversion',
        },
      },
      dialog: {
        convert: {
          title: 'Config Conversion',
          path_label: 'system.prop File Path',
          path_placeholder: 'Enter absolute path of system.prop',
          default_path_tip: 'Default Path: /data/adb/device_faker/config/system.prop',
          btn_start: 'Start Conversion',
        },
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
        input_path: 'Please enter file path',
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
