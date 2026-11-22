//! ZCode（Z.AI 的 ZCode 桌面端 / CLI）。
//!
//! ZCode 是账号制（OAuth 登录 Z.AI），供应商连接由 ZCode 自身管理；CC Switch 只在
//! 自己的数据库里登记 ZCode 供应商（additive 模式）并展示用量，不写 ZCode 内部配置。
//! 这里仅提供配置目录定位，供「打开配置目录」等功能使用。

use std::path::PathBuf;

use crate::config::get_home_dir;

/// ZCode 的配置根目录（~/.zcode）。
pub(crate) fn get_zcode_dir() -> PathBuf {
    get_home_dir().join(".zcode")
}

/// ZCode 桌面端的设置文件（存在即视为已安装/已初始化）。
pub(crate) fn zcode_settings_path() -> PathBuf {
    get_zcode_dir().join("v2").join("setting.json")
}
