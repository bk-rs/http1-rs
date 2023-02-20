//
pub enum DecoderBody {
    Completed(Vec<u8>),
    Partial(Vec<u8>),
}

pub enum EncoderBody {
    Completed(Vec<u8>),
    Partial(Vec<u8>),
}
