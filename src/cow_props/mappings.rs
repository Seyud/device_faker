//! `/proc/self/maps` 属性区映射收集与路径判定。

// ── 映射收集 ───────────────────────────────────────────────────────────────

pub(super) struct PropAreaMapping {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) path: String,
    pub(super) offset: u64,
}

pub(super) fn collect_prop_area_mappings() -> Vec<PropAreaMapping> {
    let Ok(maps) = std::fs::read_to_string("/proc/self/maps") else {
        return vec![];
    };

    let mut result = vec![];
    for line in maps.lines() {
        let mut ws = line.split_whitespace();
        let Some(range) = ws.next() else { continue };
        let Some(_perms) = ws.next() else { continue };
        let Some(off_str) = ws.next() else { continue };
        let Some(_dev) = ws.next() else { continue };
        let Some(_inode) = ws.next() else { continue };
        let Some(path) = ws.next() else { continue };

        if !path.starts_with("/dev/__properties__/") {
            continue;
        }

        let Some((start_s, end_s)) = range.split_once('-') else {
            continue;
        };
        let Ok(start) = usize::from_str_radix(start_s, 16) else {
            continue;
        };
        let Ok(end) = usize::from_str_radix(end_s, 16) else {
            continue;
        };
        let Ok(offset) = u64::from_str_radix(off_str, 16) else {
            continue;
        };

        result.push(PropAreaMapping {
            start,
            end,
            path: path.to_string(),
            offset,
        });
    }
    result
}

/// 判断路径是否为 build 相关的 prop_area。
pub(super) fn is_build_area(path: &str) -> bool {
    path.contains("build_prop")
        || path.contains("build_odm_prop")
        || path.contains("build_vendor_prop")
        || path.contains("default_prop")
}

/// 路径是否命中 `hide_maps` 配置的模式。
pub(super) fn path_is_hidden(path: &str, patterns: &[String]) -> bool {
    patterns
        .iter()
        .any(|p| !p.is_empty() && path.contains(p.as_str()))
}
