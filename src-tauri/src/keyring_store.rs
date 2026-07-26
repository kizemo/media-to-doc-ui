//! OS keyring access for LLM profile API keys.
//!
//! 设计(spec §4):
//! - service:固定 `"media-to-doc-ui"`,所有 profile 共用
//! - username:`profile:<name>`,每个 profile 一个 key
//! - password:用户填的 API key(明文存 OS keyring,keyring 自身加密)
//!
//! 平台行为:
//! - Windows:Credential Manager(WDPAPI,按用户存储,无需 admin)
//! - Mac:Keychain
//! - Linux:gnome-keyring / kwallet / secret-service daemon
//!
//! 错误一律 `Result<_, String>`,前缀 `KEYRING_ERROR:` 便于上游 grep。

use keyring::Entry;

pub const SERVICE_NAME: &str = "media-to-doc-ui";

fn entry(profile_name: &str) -> Result<Entry, String> {
    Entry::new(SERVICE_NAME, &format!("profile:{profile_name}"))
        .map_err(|e| format!("KEYRING_ERROR: 创建 entry 失败: {e}"))
}

/// 读 profile 的 API key。key 不存在时返回 `KEYRING_ERROR: NoEntry`。
pub fn read_key(profile_name: &str) -> Result<String, String> {
    let e = entry(profile_name)?;
    e.get_password()
        .map_err(|e| format!("KEYRING_ERROR: 读 key 失败: {e}"))
}

/// 写 profile 的 API key。覆盖已存在的同名 key。
pub fn write_key(profile_name: &str, key: &str) -> Result<(), String> {
    let e = entry(profile_name)?;
    e.set_password(key)
        .map_err(|e| format!("KEYRING_ERROR: 写 key 失败: {e}"))
}

/// 删 profile 的 API key。key 不存在视为成功(idempotent)。
pub fn delete_key(profile_name: &str) -> Result<(), String> {
    let e = entry(profile_name)?;
    match e.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("KEYRING_ERROR: 删 key 失败: {e}")),
    }
}

/// 列出所有 profile 名字(从 keyring username 提取 `<name>` 部分)。
///
/// 注意:keyring crate v3 没有原生的 list API,采用 platform-specific 探测:
/// - 失败时返回空 Vec(不报错)— 上层应读 metadata JSON 拿到 profile 列表。
/// - 本函数保留供将来扩展(W15-B 可能用到)。
pub fn list_profile_names() -> Result<Vec<String>, String> {
    // keyring v3 不支持跨平台 list;返回空 Vec 是安全 fallback。
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 集成测试用 profile 名(避免污染用户真实 keyring)。
    /// 每个测试用**独立** profile name,防止 OS keyring 共享状态在并行
    /// cargo test 下 race(Windows Credential Manager / DPAPI 写后立刻读
    /// 偶发返回旧值;若多测试同 entry 并发会拿到对方写前的值)。
    const TEST_PROFILE_RW: &str = "__w15a_test_write_then_read__";
    const TEST_PROFILE_OVERWRITE: &str = "__w15a_test_overwrite__";
    const TEST_PROFILE_DELETE: &str = "__w15a_test_delete__";
    const TEST_PROFILE_MISSING: &str = "__w15a_definitely_nonexistent__";

    #[test]
    fn write_then_read_returns_same_key() {
        // 先清理
        let _ = delete_key(TEST_PROFILE_RW);
        // 写
        write_key(TEST_PROFILE_RW, "sk-test-1234567890").expect("write 失败");
        // 读
        let got = read_key(TEST_PROFILE_RW).expect("read 失败");
        assert_eq!(got, "sk-test-1234567890");
        // 清理
        delete_key(TEST_PROFILE_RW).expect("delete 失败");
    }

    #[test]
    fn read_nonexistent_returns_error_with_prefix() {
        // 确保不存在
        let _ = delete_key(TEST_PROFILE_MISSING);
        let result = read_key(TEST_PROFILE_MISSING);
        assert!(result.is_err(), "读不存在的 key 应报错");
        let err = result.unwrap_err();
        assert!(
            err.starts_with("KEYRING_ERROR:"),
            "错误前缀应是 KEYRING_ERROR:, 实际: {err}"
        );
    }

    #[test]
    fn write_overwrites_existing_key() {
        let _ = delete_key(TEST_PROFILE_OVERWRITE);
        write_key(TEST_PROFILE_OVERWRITE, "first-key").unwrap();
        write_key(TEST_PROFILE_OVERWRITE, "second-key").unwrap();
        let got = read_key(TEST_PROFILE_OVERWRITE).unwrap();
        assert_eq!(got, "second-key", "二次写应覆盖");
        delete_key(TEST_PROFILE_OVERWRITE).unwrap();
    }

    #[test]
    fn delete_existing_returns_ok() {
        let _ = delete_key(TEST_PROFILE_DELETE);
        write_key(TEST_PROFILE_DELETE, "to-be-deleted").unwrap();
        let result = delete_key(TEST_PROFILE_DELETE);
        assert!(result.is_ok(), "删存在的 key 应成功");
        // 再删应 idempotent
        let result2 = delete_key(TEST_PROFILE_DELETE);
        assert!(result2.is_ok(), "再删不存在的 key 应 idempotent 成功");
    }

    #[test]
    fn list_profile_names_returns_vec() {
        // 不验证具体内容(keyring v3 不支持 list),只验证函数签名 + 返回 Vec。
        let result = list_profile_names();
        assert!(result.is_ok(), "list 应返回 Ok");
        let _names: Vec<String> = result.unwrap();
    }
}