pub trait InFramer {
    fn decode_incoming(&mut self, bytes: &[u8]) -> Vec<Vec<u8>>;
}

pub trait OutFramer {
    fn encode_outgoing(&self, bytes: &[u8]) -> Vec<Vec<u8>>;
}

pub type BoxedInFramer = Box<dyn InFramer + Send + Sync + 'static>;
pub type BoxedOutFramer = Box<dyn OutFramer + Send + Sync + 'static>;
