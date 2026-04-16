#!/system/bin/sh

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
    return 0
}

ui_print "- 安装 Device Faker 模块"
mkdir -p "$CONFIG_DIR"
chmod 755 "$CONFIG_DIR"

SHOULD_COPY_CONFIG=true

if [ -f "$CONFIG_FILE" ]; then
    ui_print "- 检测到已有配置文件"
    ui_print "- 请选择配置文件处理方式："
    ui_print "  [音量+] 使用模块默认配置（备份原有配置）"
    ui_print "  [音量-] 使用现有配置"
    ui_print "- 10秒内未选择将使用模块默认配置"

    if chooseport; then
        ui_print "- 已选择：使用模块默认配置"
        BACKUP_FILE="$CONFIG_DIR/$(date +%Y%m%d_%H%M%S)_config.toml.bak"
        ui_print "- 备份旧配置到: $BACKUP_FILE"
        cp -f "$CONFIG_FILE" "$BACKUP_FILE"
        chmod 644 "$CONFIG_FILE" "$BACKUP_FILE"
        chcon u:object_r:system_file:s0 "$CONFIG_FILE" "$BACKUP_FILE" 2>/dev/null || true
    else
        ui_print "- 已选择：使用现有配置"
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

ui_print "- 配置文件位置: $CONFIG_FILE"
ui_print "- 编辑配置文件添加需要伪装的应用"
ui_print "- 修改配置后无需重启，仅需重启对应应用"

set_perm_recursive "$MODPATH" 0 0 0755 0644
set_perm_recursive "$MODPATH/zygisk" 0 0 0755 0644
set_perm $MODPATH/bin/device_faker_cli 0 0 0755

ui_print "- 安装完成！"
ui_print "- 重启设备后生效"
