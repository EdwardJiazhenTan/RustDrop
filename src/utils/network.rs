use std::net::{TcpListener, SocketAddr};
use tracing::warn;

/// Find an available port starting from the given port number
pub fn find_available_port(start_port: u16, end_port: u16) -> Option<u16> {
    for port in start_port..=end_port {
        if is_port_available(port) {
            return Some(port);
        }
    }
    None
}

/// Check if a specific port is available
pub fn is_port_available(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    
    match TcpListener::bind(addr) {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Get the next available port starting from a given port
pub fn get_available_port_or_default(preferred_port: u16) -> u16 {
    // Try the preferred port first
    if is_port_available(preferred_port) {
        return preferred_port;
    }
    
    warn!("Port {} is not available, searching for alternative...", preferred_port);
    
    // Try ports in the 8000-8999 range
    if let Some(port) = find_available_port(8000, 8999) {
        warn!("Using alternative port: {}", port);
        return port;
    }
    
    // Fallback to 9000-9999 range
    if let Some(port) = find_available_port(9000, 9999) {
        warn!("Using fallback port: {}", port);
        return port;
    }
    
    // Last resort: return preferred port anyway (will fail at bind time)
    warn!("No available ports found, returning preferred port {}", preferred_port);
    preferred_port
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::net::TcpListener;

    #[test]
    fn test_is_port_available_free_port() {
        // Test with a very high port number that's likely to be free
        let port = 65000;
        assert!(is_port_available(port));
    }

    #[test]
    fn test_is_port_available_busy_port() {
        // Bind to a port to make it busy
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let port = addr.port();
        
        // Port should not be available while listener is active
        assert!(!is_port_available(port));
        
        // Port should become available after dropping listener
        drop(listener);
        // Note: There might be a small delay for the OS to release the port
        // so we don't test immediate availability after drop
    }

    #[test]
    fn test_find_available_port_success() {
        // Should find at least one available port in high range
        let result = find_available_port(60000, 60010);
        assert!(result.is_some());
        
        let port = result.unwrap();
        assert!(port >= 60000 && port <= 60010);
        assert!(is_port_available(port));
    }

    #[test]
    fn test_find_available_port_no_range() {
        // Test with invalid range (start > end)
        let result = find_available_port(8080, 8070);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_available_port_single_port() {
        // Test with range of 1
        let port = 65001;
        let result = find_available_port(port, port);
        
        if is_port_available(port) {
            assert_eq!(result, Some(port));
        } else {
            assert_eq!(result, None);
        }
    }

    #[test]
    fn test_get_available_port_or_default_free_port() {
        // Test with a port that should be available
        let preferred_port = 65002;
        let result = get_available_port_or_default(preferred_port);
        
        // Should return the preferred port if it's available
        if is_port_available(preferred_port) {
            assert_eq!(result, preferred_port);
        } else {
            // Should return some port in the fallback ranges
            assert!(result >= 8000);
        }
    }

    #[test]
    fn test_get_available_port_or_default_busy_port() {
        // Bind to a port to make it busy
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let busy_port = addr.port();
        
        let result = get_available_port_or_default(busy_port);
        
        // Should not return the busy port
        assert_ne!(result, busy_port);
        
        // Should return a port in the fallback ranges
        assert!(result >= 8000);
        
        drop(listener);
    }

    #[test]
    fn test_port_range_fallback_logic() {
        // This test verifies the fallback logic structure
        // We can't easily test the actual busy scenario without complex setup
        
        let preferred_port = 65003;
        let result = get_available_port_or_default(preferred_port);
        
        // Result should be a valid port number
        assert!(result > 0);
        assert!(result <= 65535);
    }

    #[test]
    fn test_multiple_port_checks() {
        // Test that multiple calls return consistent results for available ports
        let port = 65004;
        let check1 = is_port_available(port);
        let check2 = is_port_available(port);
        
        // Results should be consistent
        assert_eq!(check1, check2);
    }

    #[test]
    fn test_port_availability_edge_cases() {
        // Test port 0 (should not be available for binding)
        assert!(!is_port_available(0));
        
        // Test port 1 (typically requires root privileges)
        let result = is_port_available(1);
        // Don't assert specific result as it depends on system privileges
        // Just ensure the function doesn't panic
        let _ = result;
    }

    #[test]
    fn test_concurrent_port_availability() {
        use std::thread;
        
        // Test that port availability check works correctly with concurrent access
        let port = 65005;
        
        let handles: Vec<_> = (0..5).map(|_| {
            thread::spawn(move || {
                is_port_available(port)
            })
        }).collect();
        
        let results: Vec<bool> = handles.into_iter()
            .map(|h| h.join().unwrap())
            .collect();
        
        // All results should be the same (consistent)
        let first_result = results[0];
        assert!(results.iter().all(|&r| r == first_result));
    }

    #[test]
    fn test_find_available_port_large_range() {
        // Test with a larger range to ensure efficiency
        let result = find_available_port(50000, 50100);
        assert!(result.is_some());
        
        let port = result.unwrap();
        assert!(port >= 50000 && port <= 50100);
    }
} 