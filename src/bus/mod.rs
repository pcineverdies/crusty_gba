/// bus::MemoryRequest
///
/// structure to represent a request towards the bus
pub struct MemoryRequest {
    pub address: u32,
}

/// bus::MemoryResponse
///
/// structure to represent a response from the bus to a memory request
pub struct MemoryResponse {
    pub data: u32,
}
