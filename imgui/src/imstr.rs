pub struct NulError;
use std::os::raw::c_char;

pub struct ImStr {
    inner: [u8],
}

impl ImStr {
    #[inline]
    pub unsafe fn from_ptr<'a>(ptr: *const std::os::raw::c_char) -> &'a ImStr {
        ImStr::from_bytes_with_nul_unchecked(std::ffi::CStr::from_ptr(ptr).to_bytes())
    }

    pub fn from_bytes_with_nul(bytes: &[u8]) -> Result<&ImStr, NulError> {
        unsafe {
            std::ffi::CStr::from_bytes_with_nul(bytes)
                .map(|s| ImStr::from_bytes_with_nul_unchecked(s.to_bytes()))
                .map_err(|_| NulError)
        }
    }

    /// Returns the length of the string (does not including the NUL byte)
    pub fn len(&self) -> usize {
        self.inner.len() - 1
    }

    #[inline]
    pub unsafe fn from_bytes_with_nul_unchecked(bytes: &[u8]) -> &ImStr {
        &*(bytes as *const [u8] as *const ImStr)
    }

    #[inline]
    pub fn to_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.to_bytes())
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

    #[inline]
    pub unsafe fn begin(&self) -> *const c_char { self.inner.as_ptr() as *const _ } 
    #[inline]
    pub unsafe fn end(&self) -> *const c_char { self.inner.as_ptr().add(self.inner.len() - 1) as *const _ }
    #[inline]
    pub fn as_ptr(&self) -> *const c_char { self.inner.as_ptr() as *const _ }

    #[inline]
    pub unsafe fn begin_mut(&mut self) -> *mut c_char { self.inner.as_mut_ptr() as *mut _ } 
    #[inline]
    pub unsafe fn end_mut(&mut self) -> *mut c_char { self.inner.as_mut_ptr().add(self.inner.len() - 1) as *mut _ }
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut c_char { self.inner.as_mut_ptr() as *mut _}
}

pub struct ImStrBuf {
    inner: [u8],
}

impl ImStrBuf {
    #[inline]
    pub fn from_bytes(bytes: &mut [u8]) -> &mut ImStrBuf {
        unsafe {
            &mut *(bytes as *mut [u8] as *mut ImStrBuf)
        }
    }

    /// Returns the index of the first NUL byte. This will return the length of the string buffer
    /// if no NUL byte is found.
    #[inline]
    pub fn find_first_nul(&self) -> usize {
        self.inner.iter().position(|&b| b == b'0').unwrap_or(self.inner.len())
    }

    /// Returns a reference to the first C-String contained in this buffer.
    #[inline]
    pub fn to_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.inner[0..self.str_len()])
    }

    /// Returns the length of the first C-String contained in this buffer.
    #[inline]
    pub fn str_len(&self) -> usize {
        self.find_first_nul()
    }

    /// This returns the length of the entire buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub unsafe fn begin(&self) -> *const c_char { self.inner.as_ptr() as *const _ } 
    #[inline]
    pub unsafe fn end(&self) -> *const c_char { self.inner.as_ptr().add(self.inner.len() - 1) as *const _ }
    #[inline]
    pub fn as_ptr(&self) -> *const c_char { self.inner.as_ptr() as *const _ }

    #[inline]
    pub unsafe fn begin_mut(&mut self) -> *mut c_char { self.inner.as_mut_ptr() as *mut _ } 
    #[inline]
    pub unsafe fn end_mut(&mut self) -> *mut c_char { self.inner.as_mut_ptr().add(self.inner.len() - 1) as *mut _ }
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut c_char { self.inner.as_mut_ptr() as *mut _}
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
    pub unsafe fn from_vec_unchecked(mut v: Vec<u8>) -> ImString {
        if v.len() == 0 || v[v.len() - 1] != 0 {
            v.push(0);
        }
        ImString { inner: v }
    }

    #[inline]
    pub fn into_bytes_with_nul(self) -> Vec<u8> {
        self.inner
    }

    #[inline]
    pub fn into_bytes(mut self) -> Vec<u8> {
        let _ = self.inner.pop();
        self.inner
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner[0..self.inner.len() - 1]
    }

    #[inline]
    pub fn as_bytes_with_nul(&self) -> &[u8] {
        &self.inner[0..]
    }

    #[inline]
    pub unsafe fn begin(&self) -> *const c_char { self.inner.as_ptr() as *const _ } 
    #[inline]
    pub unsafe fn end(&self) -> *const c_char { self.inner.as_ptr().offset(self.inner.len() as isize) as *const _ }
    #[inline]
    pub fn as_ptr(&self) -> *const c_char { self.inner.as_ptr() as *const _ }

    #[inline]
    pub unsafe fn begin_mut(&mut self) -> *mut c_char { self.inner.as_mut_ptr() as *mut _ } 
    #[inline]
    pub unsafe fn end_mut(&mut self) -> *mut c_char { self.inner.as_mut_ptr().offset(self.inner.len() as isize) as *mut _ }
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut c_char { self.inner.as_mut_ptr() as *mut _ }
}
