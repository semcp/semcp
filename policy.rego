# Package declaration for snpx policies
package snpx.policy

# Imports
import future.keywords.contains
import future.keywords.if
import future.keywords.in

# Default to deny (secure by default)
default allow := false

# Allow access if all conditions are met
allow {
    allow_filesystem
    allow_network
    allow_capabilities
}

# Filesystem access rules
allow_filesystem {
    not deny_filesystem_access(input.path)
    allow_filesystem_access(input.path)
}

allow_filesystem_access(path) {
    # Path is in allowed paths
    allowed_paths := [
        "/usr/local/lib/node_modules",
        "/root/.npm",
        "/tmp",
        "/var/tmp"
    ]
    
    startswith_any(path, allowed_paths)
}

deny_filesystem_access(path) {
    # Path is in blocked paths
    blocked_paths := [
        "/proc/sys",
        "/sys/firmware",
        "/dev/mem",
        "/dev/kmem"
    ]
    
    startswith_any(path, blocked_paths)
}

# Network access rules
allow_network {
    allow_network_access(input.domain)
    not deny_network_port(input.port)
}

allow_network_access(domain) {
    # Domain is in allowed domains
    allowed_domains := [
        "registry.npmjs.org",
        "github.com",
        "api.github.com",
        "nodejs.org"
    ]
    
    domain_matches(domain, allowed_domains)
}

deny_network_port(port) {
    # Port is in blocked ports
    blocked_ports := [
        "22",
        "3389",
        "5432",
        "3306"
    ]
    
    port == blocked_ports[_]
}

# Capability management rules
allow_capabilities {
    acceptable_capabilities(input.capabilities)
}

acceptable_capabilities(caps) {
    required_drop := ["ALL"]
    allowed_add := ["SETUID", "SETGID", "DAC_OVERRIDE"]
    
    # All required capabilities are dropped
    all_required_dropped(caps.drop, required_drop)
    
    # All added capabilities are in the allowed list
    all_adds_allowed(caps.add, allowed_add)
}

# Helper functions
startswith_any(str, prefixes) {
    startswith(str, prefixes[_])
}

domain_matches(domain, allowed) {
    allowed[_] == domain
}

domain_matches(domain, allowed) {
    endswith(domain, concat(".", allowed[_]))
}

all_required_dropped(dropped, required) {
    # All required capabilities must be in the dropped list
    required_set := {x | x := required[_]}
    dropped_set := {x | x := dropped[_]}
    
    required_set.issubset(dropped_set)
}

all_adds_allowed(added, allowed) {
    # All added capabilities must be in the allowed list
    added_set := {x | x := added[_]}
    allowed_set := {x | x := allowed[_]}
    
    added_set.issubset(allowed_set)
}

# Test inputs for debugging
test_filesystem_allowed {
    allow_filesystem_access("/tmp/myfile")
}

test_filesystem_denied {
    deny_filesystem_access("/proc/sys/kernel")
}

test_network_allowed {
    allow_network_access("registry.npmjs.org")
}

test_network_port_denied {
    deny_network_port("22")
}