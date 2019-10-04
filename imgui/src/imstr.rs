pub struct NulError;

pub struct ImStr {
    inner: [u8],
}

impl ImStr {
    pub fn from_bytes(bytes: &[u8]) -> Result<&ImStr, NulError> {
        unsafe {
            std::ffi::CStr::from_bytes_with_nul(bytes)
                .map(|s| ImStr::from_bytes_unchecked(s.to_bytes()))
                .map_err(|_| NulError)
        }
    }

    /// Returns the length of the string (does not including the NUL byte)
    pub fn len(&self) -> usize {
        self.inner.len() - 1
    }

    #[inline]
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> &ImStr {
        &*(bytes as *const [u8] as *const ImStr)
    }

    /// Converts this to a byte slice. The returned sliec will **not** include the NUL byte.
    #[inline]
    pub fn to_bytes(&self) -> &[u8] {
        let bytes = self.to_bytes_with_nul();
        &bytes[..bytes.len() - 1]
    }

    /// Converts this to a byte slice. The returned sliec **will** include the NUL byte.
    #[inline]
    pub fn to_bytes_with_nul(&self) -> &[u8] {
        unsafe { &*(&self.inner as *const [std::os::raw::c_uchar] as *const [u8]) }
    }
}

pub struct ImString{
    inner: Vec<u8>
}

impl ImString {
    pub fn new<T: Into<Vec<u8>>>(t: T) -> Result<ImString, NulError> {
        unsafe {
            std::ffi::CString::new(t)
                .map(|s| ImString::from_vec_unchecked(s.into_bytes()))
                .map_err(|_| NulError)
        }
    }

    #[inline]
    pub unsafe fn from_vec_unchecked(v: Vec<u8>) -> ImString {
        if v.len() == 0 || v[v.len() - 1] != 0 {
            v.push(0);
        }
        ImString(v)
    }
}
