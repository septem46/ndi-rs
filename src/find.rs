use super::internal::bindings::*;
use super::*;
use std::ffi::CString;
use std::time::Instant;

/// Builder for [`Find`] struct
#[derive(Debug, Clone)]
pub struct FindBuilder {
    show_local_sources: Option<bool>,
    groups: Option<String>,
    extra_ips: Option<String>,
}

impl FindBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            show_local_sources: None,
            groups: None,
            extra_ips: None,
        }
    }

    /// Tells the finder whether it should locate and report NDI send sources
    /// that are running on the current local machine.
    ///
    /// default: (true)
    pub fn show_local_sources(mut self, show_local_sources: bool) -> Self {
        self.show_local_sources = Some(show_local_sources);
        self
    }
    /// Specifies groups for which this NDI finder will report sources.
    ///
    /// Groups are sets of NDI sources. Any source can be part of any
    /// number of groups, and groups are comma-separated. For instance
    /// "cameras,studio 1,10am show" would place a source in the three groups named.
    /// On the finding side, you can specify which groups to look for and also look in
    /// multiple groups. If the group is NULL then the system default groups will be used.
    pub fn groups(mut self, groups: String) -> Self {
        self.groups = Some(groups);
        self
    }

    /// specify a comma separated list of IP addresses that will be
    /// queried for NDI sources and added to the list reported by NDI find.
    ///
    /// These IP addresses need not be on the local network, and can be in any IP visible
    /// range. NDI find will be able to find and report any number of NDI sources
    /// running on remote machines, and will correctly observe them coming online and going offline.
    pub fn extra_ips(mut self, extra_ips: String) -> Self {
        self.extra_ips = Some(extra_ips);
        self
    }

    /// Build an instance of [`Find`]
    pub fn build(self) -> Result<Find, NDIError> {
        // from default c++ constructor in Processing.NDI.Find.h
        let mut settings = NDIlib_find_create_t {
            show_local_sources: true,
            p_groups: NULL as _,
            p_extra_ips: NULL as _,
        };

        if let Some(show_local_sources) = self.show_local_sources {
            settings.show_local_sources = show_local_sources;
        }

        if let Some(groups) = self.groups {
            let cstr = CString::new(groups).unwrap();
            settings.p_groups = cstr.into_raw();
        }

        if let Some(extra_ips) = self.extra_ips {
            let cstr = CString::new(extra_ips).unwrap();
            settings.p_extra_ips = cstr.into_raw();
        }

        Find::with_settings(settings)
    }
}

/// A struct to locate sources available on the network
///
/// Normally used in conjunction with [`Recv`].
/// Internally, it uses a cross-process P2P mDNS implementation to locate sources on the network. (It commonly takes
/// a few seconds to locate all of the sources available, since this requires other running machines to send response
/// messages.)
pub struct Find {
    p_instance: NDIlib_find_instance_t,
}

unsafe impl core::marker::Send for Find {}
unsafe impl core::marker::Sync for Find {}

impl Find {
    /// Create a new instance with default constructor
    pub fn new() -> Result<Self, NDIError> {
        let p_instance = unsafe { NDIlib_find_create_v2(NULL as _) };
        if p_instance.is_null() {
            return Err(NDIError::FindCreateError);
        };

        Ok(Self { p_instance })
    }

    fn with_settings(settings: NDIlib_find_create_t) -> Result<Self, NDIError> {
        let p_instance = unsafe { NDIlib_find_create_v2(&settings) };
        if p_instance.is_null() {
            return Err(NDIError::FindCreateError);
        };

        Ok(Self { p_instance })
    }

    /// List current sources
    pub fn current_sources(&self, timeout_ms: u128) -> Result<Vec<Source>, NDIError> {
        let mut no_sources = 0;
        let mut p_sources: *const NDIlib_source_t = NULL as _;
        let start = Instant::now();
        while no_sources == 0 {
            // timeout if it takes an unreasonable amount of time
            if Instant::now().duration_since(start).as_millis() > timeout_ms {
                return Err(NDIError::FindSourcesTimeout);
            }

            p_sources =
                unsafe { NDIlib_find_get_current_sources(self.p_instance, &mut no_sources) };
        }

        let mut sources: Vec<Source> = vec![];
        for _ in 0..no_sources {
            sources.push(Source::from_binding(unsafe { *p_sources }));
            p_sources = unsafe { p_sources.add(1) };
        }

        Ok(sources)
    }
}

impl Drop for Find {
    fn drop(&mut self) {
        unsafe { NDIlib_find_destroy(self.p_instance) };
    }
}
