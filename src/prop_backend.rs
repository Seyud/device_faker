//! 运行期定位 bionic 的属性后端结构，用于「私有副本」。
//!
//! 任一环节失败都返回 `Err`，由调用方降级回原地 COW（并打 WARN 说明原因）。

use std::collections::HashSet;

use anyhow::{Context, bail};
use log::{info, warn};

/// 属性区映射路径前缀。
const PROP_PREFIX: &str = "/dev/__properties__/";

/// 在 `ContextsSerialized` 对象里搜索 context 数组字段的窗口大小。
/// 只用于限定扫描范围，具体字段偏移仍由扫描 + 校验得出。
const SCAN_WINDOW: usize = 0x400;

/// `ContextNode` 元素大小的候选值（随 libc 版本变，逐个验证）。
const STRIDE_CANDIDATES: &[usize] = &[0x28, 0x20, 0x30, 0x18, 0x38, 0x40, 0x48, 0x50];

/// 探测「某个 key 实际被路由到哪个区」用的 key（`ro.*` 一定存在）。
const PROBE_KEYS: &[&str] = &[
    "ro.build.id",
    "ro.product.model",
    "ro.hardware",
    "ro.build.version.sdk",
];

// ── /proc/self/maps ──────────────────────────────────────────────────────────

struct Region {
    start: usize,
    end: usize,
    offset: u64,
    writable: bool,
    exec: bool,
    /// `major:minor`，与 `inode` 一起标识映射来源的文件
    dev: String,
    inode: u64,
    path: String,
}

struct Maps {
    regions: Vec<Region>,
}

impl Maps {
    fn load() -> Self {
        let mut regions = Vec::new();
        if let Ok(text) = std::fs::read_to_string("/proc/self/maps") {
            for line in text.lines() {
                if let Some(r) = Region::parse(line) {
                    regions.push(r);
                }
            }
        }
        Self { regions }
    }

    /// `[addr, addr+len)` 是否完整落在某个映射内。
    fn readable(&self, addr: usize, len: usize) -> bool {
        if addr == 0 {
            return false;
        }
        let Some(end) = addr.checked_add(len) else {
            return false;
        };
        self.regions.iter().any(|r| addr >= r.start && end <= r.end)
    }

    fn region_of(&self, addr: usize) -> Option<&Region> {
        self.regions
            .iter()
            .find(|r| addr >= r.start && addr < r.end)
    }

    /// 本进程所有 `/dev/__properties__/*` 映射的起始地址。
    /// bionic 的 `ContextNode::prop_area_` 就等于映射起始地址（file offset 0）。
    fn prop_area_starts(&self) -> HashSet<usize> {
        self.regions
            .iter()
            .filter(|r| r.path.starts_with(PROP_PREFIX) && r.offset == 0)
            .map(|r| r.start)
            .collect()
    }
}

impl Region {
    fn parse(line: &str) -> Option<Self> {
        let mut ws = line.split_whitespace();
        let range = ws.next()?;
        let perms = ws.next()?;
        let offset = ws.next()?;
        let dev = ws.next()?;
        let inode = ws.next()?;
        let path = ws.next().unwrap_or("");
        let (start_s, end_s) = range.split_once('-')?;
        Some(Self {
            start: usize::from_str_radix(start_s, 16).ok()?,
            end: usize::from_str_radix(end_s, 16).ok()?,
            offset: u64::from_str_radix(offset, 16).ok()?,
            writable: perms.as_bytes().get(1) == Some(&b'w'),
            exec: perms.as_bytes().get(2) == Some(&b'x'),
            dev: dev.to_string(),
            inode: inode.parse::<u64>().ok()?,
            path: path.to_string(),
        })
    }

    /// 与另一个映射是否来自同一个文件（同一 `dev:inode`）。
    fn same_file(&self, other: &Self) -> bool {
        self.inode != 0 && self.dev == other.dev && self.inode == other.inode
    }
}

// ── 裸内存读写 ───────────────────────────────────────────────────────────────

/// # Safety
/// `addr` 必须落在可读映射内且长度足够（调用方用 `Maps::readable` 校验）。
unsafe fn read_u8(addr: usize) -> u8 {
    unsafe { (addr as *const u8).read() }
}

/// # Safety
/// 同 [`read_u8`]，另需 4 字节对齐或使用非对齐读。
unsafe fn read_u32(addr: usize) -> u32 {
    unsafe { (addr as *const u32).read_unaligned() }
}

/// # Safety
/// 同 [`read_u8`]，另需 8 字节对齐或使用非对齐读。
unsafe fn read_u64(addr: usize) -> u64 {
    unsafe { (addr as *const u64).read_unaligned() }
}

/// # Safety
/// `addr` 必须落在**可写**映射内。
unsafe fn write_u64(addr: usize, value: u64) {
    unsafe { (addr as *mut u64).write_unaligned(value) }
}

// ── AArch64 指令解码（只覆盖本方案需要的 4 种）──────────────────────────────

/// `ADR Xd, label`：`0b0xx10000 immlo(2) immhi(19) Rd(5)`，
/// 目标 = PC + 符号扩展的 21 位立即数。
fn decode_adr(insn: u32, pc: usize) -> Option<usize> {
    if insn & 0x9f00_0000 != 0x1000_0000 {
        return None;
    }
    let immlo = ((insn >> 29) & 0x3) as i64;
    let immhi = ((insn >> 5) & 0x7_ffff) as i64;
    let imm = (((immhi << 2) | immlo) << 43) >> 43;
    Some(pc.wrapping_add(imm as usize))
}

/// `B label`：`0b000101 imm26`，目标 = PC + 符号扩展的 imm26 × 4。
fn decode_b(insn: u32, pc: usize) -> Option<usize> {
    if insn & 0xfc00_0000 != 0x1400_0000 {
        return None;
    }
    let imm = (((insn & 0x03ff_ffff) as i32) << 6) >> 6;
    Some(pc.wrapping_add((imm as i64 * 4) as usize))
}

/// LDR/STR（unsigned offset 寻址）：返回 `(size, opc, imm12, rn, rt)`。
/// `size=0/opc=1` = LDRB，`size=3/opc=1` = LDR(64)。
fn decode_ldst_imm(insn: u32) -> Option<(u32, u32, u32, u32, u32)> {
    if insn & 0x3f00_0000 != 0x3900_0000 {
        return None;
    }
    Some((
        insn >> 30,
        (insn >> 22) & 0x3,
        (insn >> 10) & 0xfff,
        (insn >> 5) & 0x1f,
        insn & 0x1f,
    ))
}

// ── 结构定位 ─────────────────────────────────────────────────────────────────

/// 已解析出的迁移点。
pub struct PropBackend {
    /// `ContextNode` 数组基址
    contexts: usize,
    /// 数组元素个数（`num_contexts_`）
    count: usize,
    /// 单个 `ContextNode` 的大小
    stride: usize,
    /// `ContextNode::prop_area_` 在元素内的偏移
    pa_off: usize,
}

impl PropBackend {
    /// 运行期解析 bionic 属性后端。失败时返回 `Err`，调用方应降级。
    pub fn resolve() -> anyhow::Result<Self> {
        let maps = Maps::load();
        if maps.regions.is_empty() {
            bail!("/proc/self/maps unreadable");
        }

        let wrapper =
            unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"__system_property_find".as_ptr()) as usize };
        if wrapper == 0 {
            bail!("dlsym(__system_property_find) failed");
        }

        let singleton = find_singleton(wrapper, &maps)?;
        let (backend, backend_src) = find_backend(wrapper, singleton, &maps);
        info!("prop backend: singleton={singleton:#x} backend={backend:#x} ({backend_src})");

        let area_starts = maps.prop_area_starts();
        if area_starts.len() < 8 {
            bail!(
                "too few /dev/__properties__ mappings ({})",
                area_starts.len()
            );
        }
        let probe = probe_area_start(wrapper, &maps);
        info!(
            "prop backend: {} prop areas, probe route = {:#x?}",
            area_starts.len(),
            probe
        );

        let array = find_context_array(backend, &maps, &area_starts, probe)?;
        info!(
            "prop backend: contexts={:#x} count={} stride={:#x} pa_off={:#x}",
            array.base, array.count, array.stride, array.pa_off
        );
        Ok(Self {
            contexts: array.base,
            count: array.count,
            stride: array.stride,
            pa_off: array.pa_off,
        })
    }

    /// 是否有 context 节点引用了这个属性区。
    ///
    /// 用来在建副本**之前**判断迁移是否有意义：同一个文件在本进程里可能被映射
    /// 多次（活跃/非活跃两套 `ContextsSerialized` 各自映射一遍），只有被节点
    /// 引用的那一份才值得克隆 —— 否则克隆出来也没有指针指向它，纯属浪费
    /// （还会白白 mmap/munmap 一轮，扰动地址空间）。
    pub fn references_area(&self, addr: usize) -> bool {
        (0..self.count).any(|i| {
            let slot = self.contexts + i * self.stride + self.pa_off;
            (unsafe { read_u64(slot) }) as usize == addr
        })
    }

    /// 把所有 `prop_area_ == from` 的 context 节点改指向 `to`，返回改写个数。
    ///
    /// 改写 `to == 0` 之外的值前请确认 `to` 是一个内容自洽的属性区副本：
    /// 此后进程内所有该区的属性读取都会走副本。
    pub fn repoint_area(&self, from: usize, to: usize) -> usize {
        let mut n = 0;
        for i in 0..self.count {
            let slot = self.contexts + i * self.stride + self.pa_off;
            if unsafe { read_u64(slot) } as usize == from {
                unsafe { write_u64(slot, to as u64) };
                n += 1;
            }
        }
        n
    }
}

/// 从 `__system_property_find` 的 `adr` 指令解码静态单例地址。
///
/// 校验两条（缺一不可，避免解码到无关地址）：
/// 1. 目标落在**可写**段 —— 单例是可变静态对象，必然在 `.bss`/`.data`
///    （字符串字面量在 `.rodata`，只读，可直接排除）；
/// 2. 目标首 8 字节（vptr）指向**与包装同一个 so**的数据段（虚表在 `.data.rel.ro`）。
///
/// ⚠️ 注意不能拿 `maps` 里的**路径**做比较：单例所在的 `.bss` 在运行时是
/// 独立的匿名映射（`[anon:.bss]`），路径与 libc 的 `.text` 不同，
/// 但 vptr 所在的 `.data.rel.ro` 与 `.text` 同属 libc.so 文件 —— 故用 `dev:inode` 比对。
fn find_singleton(wrapper: usize, maps: &Maps) -> anyhow::Result<usize> {
    let code = maps
        .region_of(wrapper)
        .ok_or_else(|| anyhow::anyhow!("__system_property_find not in any mapping"))?;

    for i in 0..32 {
        let pc = wrapper + i * 4;
        if !maps.readable(pc, 4) {
            break;
        }
        let insn = unsafe { read_u32(pc) };
        let Some(target) = decode_adr(insn, pc) else {
            continue;
        };
        let Some(region) = maps.region_of(target) else {
            continue;
        };
        if !region.writable || !maps.readable(target, 16) {
            continue;
        }
        let vptr = unsafe { read_u64(target) } as usize;
        let Some(vptr_region) = maps.region_of(vptr) else {
            continue;
        };
        if !maps.readable(vptr, 8) || !vptr_region.same_file(code) {
            continue;
        }
        return Ok(target);
    }
    bail!("no usable ADR target in __system_property_find")
}

/// 定位 `ContextsSerialized` 对象（`SystemProperties::Find` 里 `+backend_off` 处）。
///
/// 返回 `(对象地址, 来源说明)`。优先按指令解码；解码失败时退回「单例本体」
/// ——`SystemProperties::AreaInit` 会把该字段写成 `this`，所以正常进程里两者相同。
fn find_backend(wrapper: usize, singleton: usize, maps: &Maps) -> (usize, &'static str) {
    if let Some(find_fn) = find_tail_call(wrapper, maps)
        && let Some((inited_off, backend_off)) = decode_find_layout(find_fn, maps)
    {
        let inited = maps.readable(singleton + inited_off, 1)
            && unsafe { read_u8(singleton + inited_off) } == 1;
        if inited {
            let backend = unsafe { read_u64(singleton + backend_off) } as usize;
            if maps.readable(backend, 16) {
                return (backend, "decoded from SystemProperties::Find");
            }
        } else {
            warn!("prop backend: inited byte at +{inited_off:#x} is not 1, ignoring decode");
        }
    }
    (singleton, "fallback: backend == singleton")
}

/// 在导出包装里找尾部无条件跳转（`b SystemProperties::Find`）。
fn find_tail_call(wrapper: usize, maps: &Maps) -> Option<usize> {
    let code = maps.region_of(wrapper)?;
    for i in 0..8 {
        let pc = wrapper + i * 4;
        if !maps.readable(pc, 4) {
            break;
        }
        let insn = unsafe { read_u32(pc) };
        let Some(target) = decode_b(insn, pc) else {
            continue;
        };
        let Some(region) = maps.region_of(target) else {
            continue;
        };
        if region.exec && region.same_file(code) {
            return Some(target);
        }
    }
    None
}

/// 解码 `SystemProperties::Find` 序言，取 `(inited_off, backend_off)`。
///
/// 两者都是「以 `this`（入口参数 x0）为基址的 load」，且 `inited_` 是
/// 紧随其后的那个字节字段（`backend_off < inited_off`）。
fn decode_find_layout(find_fn: usize, maps: &Maps) -> Option<(usize, usize)> {
    let mut inited_off = None;
    let mut backend_off = None;
    for i in 0..16 {
        let pc = find_fn + i * 4;
        if !maps.readable(pc, 4) {
            break;
        }
        let insn = unsafe { read_u32(pc) };
        let Some((size, opc, imm12, rn, _rt)) = decode_ldst_imm(insn) else {
            continue;
        };
        // 只要 load（opc=1），且基址寄存器就是入口的 this（x0）。
        if opc != 1 || rn != 0 {
            continue;
        }
        match size {
            0 => {
                inited_off.get_or_insert(imm12 as usize);
            }
            3 => {
                backend_off.get_or_insert(imm12 as usize * 8);
            }
            _ => {}
        }
    }
    let (inited_off, backend_off) = (inited_off?, backend_off?);
    if backend_off >= inited_off || inited_off == 0 {
        return None;
    }
    Some((inited_off, backend_off))
}

/// 调用 `__system_property_find` 探测某个 key 实际路由到哪个区。
fn probe_area_start(wrapper: usize, maps: &Maps) -> Option<usize> {
    type FindFn = unsafe extern "C" fn(*const libc::c_char) -> *const libc::c_void;
    let find: FindFn = unsafe { std::mem::transmute(wrapper) };
    for key in PROBE_KEYS {
        let Ok(ckey) = std::ffi::CString::new(*key) else {
            continue;
        };
        let pi = unsafe { find(ckey.as_ptr()) };
        if pi.is_null() {
            continue;
        }
        let addr = pi as usize;
        if let Some(r) = maps.region_of(addr)
            && r.path.starts_with(PROP_PREFIX)
            && r.offset == 0
        {
            return Some(r.start);
        }
    }
    None
}

struct ContextArray {
    base: usize,
    count: usize,
    stride: usize,
    pa_off: usize,
}

/// 在 `backend` 对象里扫描 context 数组。
///
/// 校验条件（缺一不可）：
/// 1. `(base, count)` 相邻，`base` 可读，`count ∈ [8, 4096]`；
/// 2. 整个数组可读；
/// 3. 每个元素的 `prop_area_` 要么为 0（尚未惰性打开），要么**恰好**是本进程
///    某个真实 `/dev/__properties__` 映射的起点；
/// 4. 非零元素 ≥ 3 且至少分布在 2 个不同区；
/// 5. 若探针拿到了路由结果，数组里必须包含该区（行为锚点）。
fn find_context_array(
    backend: usize,
    maps: &Maps,
    area_starts: &HashSet<usize>,
    probe: Option<usize>,
) -> anyhow::Result<ContextArray> {
    let mut found: Option<ContextArray> = None;
    let mut tried = 0usize;

    for field_off in (0..SCAN_WINDOW).step_by(8) {
        if !maps.readable(backend + field_off, 16) {
            break;
        }
        let base = unsafe { read_u64(backend + field_off) } as usize;
        let count = unsafe { read_u64(backend + field_off + 8) } as usize;
        if !(8..=4096).contains(&count) || !maps.readable(base, 8) {
            continue;
        }
        for &stride in STRIDE_CANDIDATES {
            let span = (count - 1) * stride + 8;
            if !maps.readable(base, span) {
                continue;
            }
            for pa_off in (0..stride).step_by(8) {
                tried += 1;
                if validate_context_array(base, count, stride, pa_off, area_starts, probe) {
                    // 取字段偏移最小者：同一个对象里更靠前的就是活跃的那个
                    // ContextsSerialized（zygote reload 用的第二个实例在后面）。
                    found = Some(ContextArray {
                        base,
                        count,
                        stride,
                        pa_off,
                    });
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        if found.is_some() {
            break;
        }
    }

    found.with_context(|| format!("no self-consistent context array in backend (tried {tried})"))
}

fn validate_context_array(
    base: usize,
    count: usize,
    stride: usize,
    pa_off: usize,
    area_starts: &HashSet<usize>,
    probe: Option<usize>,
) -> bool {
    let mut nonzero = 0usize;
    let mut distinct = HashSet::new();
    let mut has_probe = probe.is_none();
    for i in 0..count {
        let v = unsafe { read_u64(base + i * stride + pa_off) } as usize;
        if v == 0 {
            continue;
        }
        if !area_starts.contains(&v) {
            return false;
        }
        nonzero += 1;
        distinct.insert(v);
        if Some(v) == probe {
            has_probe = true;
        }
    }
    nonzero >= 3 && distinct.len() >= 2 && has_probe
}
