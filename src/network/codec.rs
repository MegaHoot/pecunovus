use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
use std::io;

/// Thin wrapper that produces/consumes raw bytes frames via LengthDelimitedCodec.
/// Serialization/deserialization of WireMessage is done in connection layer using bincode.
#[derive(Debug)]
pub struct FrameCodec {
    inner: LengthDelimitedCodec,
}

impl FrameCodec {
    pub fn new() -> Self {
        Self {
            inner: LengthDelimitedCodec::new(),
        }
    }
}

impl Decoder for FrameCodec {
    type Item = bytes::Bytes;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.inner.decode(src)? {
            Some(buf) => Ok(Some(buf.freeze())),
            None => Ok(None),
        }
    }
}

impl Encoder<bytes::Bytes> for FrameCodec {
    type Error = io::Error;

    fn encode(&mut self, item: bytes::Bytes, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.inner.encode(item, dst)
    }
}
