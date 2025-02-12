use twizzler_runtime_api::DebugRuntime;

use super::ReferenceRuntime;

// TODO: hook into dynlink for this stuff

impl DebugRuntime for ReferenceRuntime {
    fn get_library(
        &self,
        _id: twizzler_runtime_api::LibraryId,
    ) -> Option<twizzler_runtime_api::Library> {
        None
    }

    fn get_exeid(&self) -> Option<twizzler_runtime_api::LibraryId> {
        None
    }

    fn get_library_segment(
        &self,
        _lib: &twizzler_runtime_api::Library,
        _seg: usize,
    ) -> Option<twizzler_runtime_api::AddrRange> {
        None
    }

    fn get_full_mapping(
        &self,
        _lib: &twizzler_runtime_api::Library,
    ) -> Option<twizzler_runtime_api::ObjectHandle> {
        None
    }
}
