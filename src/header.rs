pub const HEADER_LEN: usize = 22; // PROPOSAL SM-D10: fixed datagram header length.
pub const MAGIC: u16 = 0x534D; // PROPOSAL SM-D10: socket-mux datagram magic.
pub const VERSION: u8 = 1; // PROPOSAL SM-D10: datagram header version.
pub const FLAG_EOS: u8 = 1; // PROPOSAL SM-D10: end-of-stream flag.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Header {
    pub flags: u8,
    pub payload_len: u16,
    pub sequence: u64,
    pub stream_offset: u64,
}

impl Header {
    pub fn new(payload_len: u16, sequence: u64, stream_offset: u64, eos: bool) -> Self {
        Self {
            flags: if eos { FLAG_EOS } else { 0 },
            payload_len,
            sequence,
            stream_offset,
        }
    }

    pub fn encode(self) -> [u8; HEADER_LEN] {
        // PROPOSAL SM-D10: encode the 22-byte header in network byte order.
        let mut out = [0; HEADER_LEN];
        out[0..2].copy_from_slice(&MAGIC.to_be_bytes());
        out[2] = VERSION;
        out[3] = self.flags;
        out[4..6].copy_from_slice(&self.payload_len.to_be_bytes());
        out[6..14].copy_from_slice(&self.sequence.to_be_bytes());
        out[14..22].copy_from_slice(&self.stream_offset.to_be_bytes());
        out
    }

    pub fn decode(datagram: &[u8]) -> Result<Self, &'static str> {
        if datagram.len() < HEADER_LEN {
            return Err("datagram is shorter than the header");
        }
        let magic = u16::from_be_bytes([datagram[0], datagram[1]]);
        if magic != MAGIC {
            return Err("bad datagram magic");
        }
        if datagram[2] != VERSION {
            return Err("unsupported datagram version");
        }
        let payload_len = u16::from_be_bytes([datagram[4], datagram[5]]);
        if datagram.len() != HEADER_LEN + usize::from(payload_len) {
            return Err("datagram length does not match payload length");
        }
        Ok(Self {
            flags: datagram[3],
            payload_len,
            sequence: u64::from_be_bytes(datagram[6..14].try_into().unwrap()),
            stream_offset: u64::from_be_bytes(datagram[14..22].try_into().unwrap()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let header = Header::new(1200, 42, 987_654_321, true);
        let mut datagram = header.encode().to_vec();
        datagram.resize(HEADER_LEN + usize::from(header.payload_len), 0);
        assert_eq!(Header::decode(&datagram), Ok(header));
        assert_eq!(&datagram[0..2], &[0x53, 0x4d]);
        assert_eq!(datagram[2], 1);
        assert_eq!(datagram[3], FLAG_EOS);
    }

    #[test]
    fn rejects_bad_magic_version_and_length() {
        let header = Header::new(3, 0, 0, false);
        let encoded = header.encode();

        let mut bad_magic = encoded;
        bad_magic[0] ^= 1;
        assert_eq!(Header::decode(&bad_magic), Err("bad datagram magic"));

        let mut bad_version = encoded;
        bad_version[2] = 2;
        assert_eq!(
            Header::decode(&bad_version),
            Err("unsupported datagram version")
        );

        assert!(Header::decode(&encoded[..21]).is_err());
        let mut datagram = encoded.to_vec();
        datagram.extend_from_slice(&[1, 2, 3]);
        assert_eq!(Header::decode(&datagram), Ok(header));
        datagram.push(4);
        assert_eq!(
            Header::decode(&datagram),
            Err("datagram length does not match payload length")
        );
    }
}
