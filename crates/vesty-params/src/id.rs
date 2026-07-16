pub type ParamId = String;
pub type Vst3ParamId = u32;

pub const VST3_PARAM_ID_ALGORITHM: &str = "vesty.vst3.param.fnv1a31-positive.v2";

pub fn stable_vst3_param_id(param_id: &str) -> Vst3ParamId {
    const FNV_OFFSET: u32 = 0x811c_9dc5;
    const FNV_PRIME: u32 = 0x0100_0193;
    let mut hash = FNV_OFFSET;
    for byte in b"vesty.vst3.param:"
        .iter()
        .copied()
        .chain(param_id.as_bytes().iter().copied())
    {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    let positive_id = hash & 0x7fff_ffff;
    if positive_id == 0 { 1 } else { positive_id }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParamHandle {
    index: usize,
}

impl ParamHandle {
    pub const INVALID_INDEX: usize = usize::MAX;

    pub const fn from_index(index: usize) -> Self {
        Self { index }
    }

    pub const fn invalid() -> Self {
        Self {
            index: Self::INVALID_INDEX,
        }
    }

    pub const fn index(self) -> usize {
        self.index
    }

    pub const fn is_invalid(self) -> bool {
        self.index == Self::INVALID_INDEX
    }
}
