#!/system/bin/sh

# Language detection: Chinese if the system locale starts with "zh", otherwise English.
LOCALE_VALUE=""
for _locale_prop in persist.sys.locale ro.product.locale persist.sys.language ro.product.locale.language; do
    _locale_val=$(getprop "$_locale_prop" 2>/dev/null)
    if [ -n "$_locale_val" ]; then
        LOCALE_VALUE="$_locale_val"
        break
    fi
done

case "$LOCALE_VALUE" in
    zh*) LANG_CN=true ;;
    *)   LANG_CN=false ;;
esac

tr_print() {
    if $LANG_CN; then
        ui_print "$1"
    else
        ui_print "$2"
    fi
}

check_zygisk() {
    # libzygisk.so: ZygiskNext etc.; libzygisk64.so: YukiZygisk
    if find /data/adb/modules /data/adb/modules_update /data/adb/ksu/lib \( -name "libzygisk.so" -o -name "libzygisk64.so" \) 2>/dev/null | grep -q .; then
        return 0
    fi

    if $LANG_CN; then
        abort "! 未检测到 Zygisk 实现。请安装 ZygiskNext 模块（不支持 Magisk 自带 Zygisk）"
    else
        abort "! No Zygisk implementation found. Please install ZygiskNext (Magisk built-in Zygisk is not supported)"
    fi
}

check_zygisk

CONFIG_DIR="/data/adb/device_faker/config"
CONFIG_FILE="$CONFIG_DIR/config.toml"

chooseport() {
    local timeout=10
    local count=0
    while [ $count -lt $timeout ]; do
        local key_event=$(timeout 1 getevent -lc 1 2>&1 | grep VOLUME | grep " DOWN")
        if [ -n "$key_event" ]; then
            echo "$key_event" | grep VOLUMEUP > /dev/null && return 0 || return 1
        fi
        count=$((count + 1))
    done
    return 1
}

tr_print "- 安装 Device Faker 模块" "- Installing Device Faker module"
mkdir -p "$CONFIG_DIR"
chmod 755 "$CONFIG_DIR"

SHOULD_COPY_CONFIG=true

if [ -f "$CONFIG_FILE" ]; then
    tr_print "- 检测到已有配置文件" "- Existing config file detected"
    tr_print "- 请选择配置文件处理方式：" "- Choose how to handle config:"
    tr_print "  [音量+] 使用模块默认配置（备份原有配置）" "  [Vol+] Use default config (backup existing)"
    tr_print "  [音量-] 使用现有配置" "  [Vol-] Keep existing config"
    tr_print "- 10秒内未选择将保留现有配置" "- Existing config will be kept if no choice within 10s"

    if chooseport; then
        tr_print "- 已选择：使用模块默认配置" "- Selected: use default config"
        BACKUP_FILE="$CONFIG_DIR/$(date +%Y%m%d_%H%M%S)_config.toml.bak"
        tr_print "- 备份旧配置到: $BACKUP_FILE" "- Backup saved to: $BACKUP_FILE"
        cp -f "$CONFIG_FILE" "$BACKUP_FILE"
        chmod 644 "$CONFIG_FILE" "$BACKUP_FILE"
        chcon u:object_r:system_file:s0 "$CONFIG_FILE" "$BACKUP_FILE" 2>/dev/null || true
    else
        tr_print "- 已选择：使用现有配置" "- Selected: keep existing config"
        SHOULD_COPY_CONFIG=false
    fi
fi

if [ "$SHOULD_COPY_CONFIG" = true ]; then
    cp -f "$MODPATH/config.toml" "$CONFIG_FILE"
    chmod 644 "$CONFIG_FILE"
    chcon u:object_r:system_file:s0 "$CONFIG_FILE" 2>/dev/null || true
fi

chcon u:object_r:system_file:s0 "$CONFIG_DIR" 2>/dev/null || true

rm -f "$MODPATH/config.toml"

tr_print "- 配置文件位置: $CONFIG_FILE" "- Config file location: $CONFIG_FILE"
tr_print "- 编辑配置文件添加需要伪装的应用" "- Edit config file to add apps for spoofing"
tr_print "- 修改配置后无需重启，仅需重启对应应用" "- No reboot needed, just restart the target app"

set_perm_recursive "$MODPATH" 0 0 0755 0644
set_perm_recursive "$MODPATH/zygisk" 0 0 0755 0644
# TODO(remove-cli): device_faker_cli 已不随模块打包（WebUI 转换已纯前端实现）。
# 若确认无人再手动使用，可连同 device_faker_cli/ crate 一起删除。

tr_print "- 安装完成！" "- Installation complete!"
tr_print "- 重启设备后生效" "- Reboot device to take effect"
