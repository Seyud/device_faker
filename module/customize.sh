#!/system/bin/sh

CONFIG_DIR="/data/adb/device_faker/config"
CONFIG_FILE="$CONFIG_DIR/config.toml"

ui_print "- 安装 Device Faker 模块"

ui_print "- 创建配置目录: $CONFIG_DIR"
mkdir -p "$CONFIG_DIR"

chmod 755 "$CONFIG_DIR"

if [ -f "$CONFIG_FILE" ]; then
    ui_print "- 检测到已有配置文件，保留现有配置"
    chmod 644 "$CONFIG_FILE"
    chcon u:object_r:system_file:s0 "$CONFIG_FILE" 2>/dev/null || true
else
    ui_print "- 复制默认配置文件"
    cp -f "$MODPATH/config.toml" "$CONFIG_FILE"
    chmod 644 "$CONFIG_FILE"
    chcon u:object_r:system_file:s0 "$CONFIG_FILE" 2>/dev/null || true
fi

chcon u:object_r:system_file:s0 "$CONFIG_DIR" 2>/dev/null || true

ui_print "- 配置文件位置: $CONFIG_FILE"
ui_print "- 请编辑配置文件添加需要伪装的应用"
ui_print "- 修改配置后无需重启，仅需重启对应应用"

set_perm_recursive "$MODPATH" 0 0 0755 0644
set_perm_recursive "$MODPATH/zygisk" 0 0 0755 0644

ui_print "- 安装完成！"
ui_print "- 重启设备后生效"
