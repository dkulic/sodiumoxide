macro_rules! stream_module (($state_name: ident,
                             $init_push_name:ident,
                             $push_name:ident,
                             $init_pull_name:ident,
                             $pull_name:ident,
                             $rekey_name: ident,
                             $keybytes:expr,
                             $headerbytes:expr,
                             $abytes:expr,
                             $tag_message: expr,
                             $tag_push: expr,
                             $tag_rekey: expr,
                             $tag_final: expr) => (

#[cfg(not(feature = "std"))] use prelude::*;
use libc::c_ulonglong;
use randombytes::randombytes_into;
use std::mem;

/// Number of bytes in a `Key`.
pub const KEYBYTES: usize = $keybytes as usize;

/// Number of bytes in a `Header`.
/// An encrypted stream starts with a short header, whose size is HEADERBYTES bytes.
/// That header must be sent/stored before the sequence of encrypted messages,
/// as it is required to decrypt the stream.
pub const HEADERBYTES: usize = $headerbytes as usize;

/// Number of added bytes. The ciphertext length is guaranteed to always be message length + ABYTES.
pub const ABYTES: usize = $abytes as usize;

/// Tag message, the most common tag, that doesn't add any information about the nature of the message.
const TAG_MESSAGE: u8 = $tag_message as u8;

/// Tag push: indicates that the message marks the end of a set of messages,
/// but not the end of the stream.
/// For example, a huge JSON string sent as multiple chunks can use this tag to indicate
/// to the application that the string is complete and that it can be decoded.
/// But the stream itself is not closed, and more data may follow.
const TAG_PUSH: u8 = $tag_push as u8;

/// Tag rekey: "forget" the key used to encrypt this message and the previous ones,
/// and derive a new secret key.
const TAG_REKEY: u8 = $tag_rekey as u8;

/// Tag final: indicates that the message marks the end of the stream
/// and erases the secret key used to encrypt the previous sequence.
const TAG_FINAL: u8 = $tag_final as u8;

/// Tag of the message. When message is encrypted the tag is attached.
/// When decrypting the tag is retrieved and may be used.
#[derive(Debug, PartialEq)]
pub enum Tag {
    /// Message, the most common tag, that doesn't add any information about the nature of the message.
    Message,

    /// Push: indicates that the message marks the end of a set of messages,
    /// but not the end of the stream.
    /// For example, a huge JSON string sent as multiple chunks can use this tag to indicate
    /// to the application that the string is complete and that it can be decoded.
    /// But the stream itself is not closed, and more data may follow.
    Push,

    /// Rekey: "forget" the key used to encrypt this message and the previous ones,
    /// and derive a new secret key.
    Rekey,

    /// Final: indicates that the message marks the end of the stream
    /// and erases the secret key used to encrypt the previous sequence.
    Final,
}

fn _tag_from_byte(tag: u8) -> Result<Tag, ()> {
    match tag {
        TAG_MESSAGE => Ok(Tag::Message),
        TAG_PUSH => Ok(Tag::Push),
        TAG_REKEY => Ok(Tag::Rekey),
        TAG_FINAL => Ok(Tag::Final),
        _ => Err(())
    }
}

new_type! {
    /// `Key` for symmetric encryption
    ///
    /// When a `Key` goes out of scope its contents
    /// will be zeroed out
    secret Key(KEYBYTES);
}

new_type! {
    /// An encrypted stream starts with a short header, whose size is HEADERBYTES bytes.
    /// That header must be sent/stored before the sequence of encrypted messages,
    /// as it is required to decrypt the stream.
    public Header(HEADERBYTES);
}

/// `gen_key()` randomly generates a key for symmetric encryption
///
/// THREAD SAFETY: `gen_key()` is thread-safe provided that you have
/// called `sodiumoxide::init()` once before using any other function
/// from sodiumoxide.
pub fn gen_key() -> Key {
    let mut key: [u8; KEYBYTES] = unsafe { mem::uninitialized() };
    randombytes_into(&mut key);
    Key(key)
}

/// `Encryptor` contains the state for multi-part (streaming) computations. This allows the caller
/// to process encryption of a sequence of multiple messages.
pub struct Encryptor($state_name);

impl Encryptor {
    /// Initializes a `Encryptor` using the `key` and an internal, automatically generated initialization vector.
    /// It then stores the stream header into header.
    /// The header must be sent/stored before the sequence of encrypted messages,
    /// as it is required to decrypt the stream.
    pub fn init(key: &Key) -> Result<(Encryptor, Header), ()> {
        unsafe {
            let mut state: $state_name = mem::uninitialized();
            let mut header: [u8; HEADERBYTES] = mem::uninitialized();
            let err = $init_push_name(&mut state, header.as_mut_ptr(), key.0.as_ptr());
            if err == 0 {
                Ok((Encryptor(state), Header(header)))
            }
            else {
                Err(())
            }
        }
    }

    /// Encrypts a message `m` using the `state` and tags it as `Message`.
    /// Additional data ad of length adlen can be included in the computation of the authentication tag.
    /// If no additional data is required, ad can be None.
    pub fn message(&mut self, m: &[u8], ad: Option<&[u8]>) -> Result<Vec<u8>, ()> {
        self._push(m, ad, TAG_MESSAGE)
    }

    /// Encrypts a message `m` using the `state` and tags it as `Push`.
    /// Additional data ad of length adlen can be included in the computation of the authentication tag.
    /// If no additional data is required, ad can be None.
    pub fn push(&mut self, m: &[u8], ad: Option<&[u8]>) -> Result<Vec<u8>, ()> {
        self._push(m, ad, TAG_PUSH)
    }

    /// Encrypts a message `m` using the `state` and issues an rekey event. Message is tagged as `Rekey`.
    /// Additional data ad of length adlen can be included in the computation of the authentication tag.
    /// If no additional data is required, ad can be None.
    pub fn rekey_message(&mut self, m: &[u8], ad: Option<&[u8]>) -> Result<Vec<u8>, ()> {
        self._push(m, ad, TAG_REKEY)
    }

    /// Encrypts a message `m` using the `state` and finalizes the secret stream.
    /// Additional data ad of length adlen can be included in the computation of the authentication tag.
    /// If no additional data is required, ad can be None.
    pub fn finalize(mut self, m: &[u8], ad: Option<&[u8]>) -> Result<Vec<u8>, ()> {
        self._push(m, ad, TAG_FINAL)
    }

    /// Explicit rekeying, updates the state, but doesn't add any information about the key change to the stream.
    /// If this function is used to create an encrypted stream, the decryption process must call that function at the exact same stream location.
    pub fn rekey(&mut self) {
        unsafe {
            $rekey_name(&mut self.0);
        }
    }

    /// Encrypts a message `m` using the `state` and the `tag`.
    /// Additional data ad of length adlen can be included in the computation of the authentication tag.
    /// If no additional data is required, ad can be None.
    fn _push(&mut self, m: &[u8], ad: Option<&[u8]>, tag: u8) -> Result<Vec<u8>, ()> {
        let (ad_p, ad_len) = ad.map(|ad| (ad.as_ptr(), ad.len() as c_ulonglong)).unwrap_or((0 as *const _, 0));
        let mut c = Vec::with_capacity(m.len() + ABYTES);
        let mut clen = c.capacity() as c_ulonglong;

        unsafe {
            let err = $push_name(&mut self.0,
                                 c.as_mut_ptr(),
                                 &mut clen,
                                 m.as_ptr(),
                                 m.len() as c_ulonglong,
                                 ad_p,
                                 ad_len,
                                 tag);
            if err != 0 {
                return Err(());
            }
            c.set_len(clen as usize);
        }
        Ok(c)
    }
}

/// `Decryptor` contains the state for multi-part (streaming) computations. This allows the caller
/// to process encryption of a sequence of multiple messages.
pub struct Decryptor {
    state: $state_name,
    flag_finalized: bool,
}

impl Decryptor {
    /// Initializes a `state` given a secret `key` and a `header`.
    /// The `key` k will not be required any more for subsequent operations.
    /// It returns Err if the header is invalid.
    pub fn init(header: &Header, key: &Key) -> Result<Decryptor, ()> {
        unsafe {
            let mut state: $state_name = mem::uninitialized();
            if $init_pull_name(&mut state, header.0.as_ptr(), key.0.as_ptr()) != 0 {
                return Err(());
            }
            Ok(Decryptor{state, flag_finalized: false})
        }
    }

    /// Verifies that c (a sequence of bytes) contains a valid ciphertext and authentication tag
    /// for the given state state and optional authenticated data ad of length adlen bytes.
    /// If the ciphertext appears to be invalid, the function returns Err.
    /// If the authentication tag appears to be correct, the decrypted message is returned with tag.
    /// Applications will typically call this function in a loop, until
    /// a message with the Tag::Final tag is found.
    pub fn decrypt(&mut self, c: &[u8], ad: Option<&[u8]>) -> Result<(Vec<u8>, Tag),()> {
        let (ad_p, ad_len) = ad.map(|ad| (ad.as_ptr(), ad.len() as c_ulonglong)).unwrap_or((0 as *const _, 0));
        let mut m = Vec::with_capacity(c.len() - ABYTES);
        let mut mlen = m.capacity() as c_ulonglong;
        let mut tag: u8 = 0;

        unsafe {
            if $pull_name(&mut self.state,
                          m.as_mut_ptr(),
                          &mut mlen,
                          &mut tag,
                          c.as_ptr(),
                          c.len() as c_ulonglong,
                          ad_p,
                          ad_len) != 0 {
                return Err(());
            }
            m.set_len(mlen as usize);
        }
        let tag = _tag_from_byte(tag)?;
        if tag == Tag::Final { self.flag_finalized = true; }
        Ok((m, tag))
    }

    /// Explicit rekeying, updates the state, but doesn't add any information about the key change to the stream.
    /// If this function is used to create an encrypted stream,
    /// the decryption process must call that function at the exact same stream location.
    pub fn rekey(&mut self) {
        unsafe {
            $rekey_name(&mut self.state);
        }
    }

    /// Check if stream is finalized.
    pub fn is_finalized(&self) -> bool {
        self.flag_finalized
    }
}

));