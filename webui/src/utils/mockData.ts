// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

// 开发环境的模拟数据
export const mockConfig = `
# 机型伪装配置文件

[templates.redmagic_9_pro]
packages = [
    "com.mobilelegends.mi",
    "com.supercell.brawlstars",
]
manufacturer = "ZTE"
brand = "nubia"
model = "NX769J"
device = "REDMAGIC 9 Pro"
product = "NX769J"
name = "NX769J"
fingerprint = "nubia/NX769J/NX769J:14/UKQ1.230917.001/20240813.173312:user/release-keys"

[[apps]]
package = "com.omarea.vtools"
manufacturer = "Xiaomi"
model = "2509FPN0BC"
brand = "Xiaomi"
device = "Xiaomi 15 Pro"
fingerprint = "Xiaomi/2509FPN0BC/Xiaomi15Pro:14/UP1A.231005.007/V816.0.3.0.VNBCNXM:user/release-keys"
`

export const mockModuleProp = `
id=device_faker
name=Device Faker
version=1.0.0
versionCode=100
author=酷安@瓦力喀/GitHub@Seyud
description=机型伪装模块
`

export const mockInstalledApps = [
  {
    packageName: 'com.android.settings',
    appName: '设置',
    versionName: '15',
    versionCode: 35,
    installed: true,
    isSystem: true,
  },
  {
    packageName: 'com.tencent.mm',
    appName: '微信',
    versionName: '8.0.40',
    versionCode: 2340,
    installed: true,
    isSystem: false,
  },
  {
    packageName: 'com.mobilelegends.mi',
    appName: 'Mobile Legends',
    versionName: '1.8.20',
    versionCode: 18200,
    installed: true,
    isSystem: false,
  },
  {
    packageName: 'com.supercell.brawlstars',
    appName: 'Brawl Stars',
    versionName: '51.170',
    versionCode: 51170,
    installed: true,
    isSystem: false,
  },
  {
    packageName: 'com.omarea.vtools',
    appName: 'Scene',
    versionName: '5.6.1',
    versionCode: 561,
    installed: true,
    isSystem: false,
  },
  {
    packageName: 'com.example.propertytest@999',
    appName: 'Property Test (XSpace)',
    versionName: '1.0',
    versionCode: 1,
    installed: true,
    isSystem: false,
  },
]
