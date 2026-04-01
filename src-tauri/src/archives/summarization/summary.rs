// builtin

// external

// internal

pub struct Summary {
    pub raw: String,
}

impl Summary {
    pub fn from_raw(s: String) -> Summary {
        Summary { raw: s }
    }
}
