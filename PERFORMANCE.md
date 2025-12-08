# Performance Comparison: Nexus Dock vs rdock

**Test Date:** December 8, 2025  
**Test Duration:** 30 seconds per application  
**Sample Interval:** 500ms (60 samples per app)

## Executive Summary

rdock demonstrates **significantly better resource efficiency** compared to Nexus Dock:

- **74.1% less private memory** usage
- **84.1% smaller binary** size
- **72.3% fewer handles** (207 vs 747)
- **62.5% fewer threads** (3 vs 8)

## Detailed Results

### Memory Usage

#### Working Set (RAM)
- **Nexus:** 21.94 MB (stable)
- **rdock:** 21.62 MB (avg, range: 21.16-21.74 MB)
- **Difference:** rdock uses 1.4% less

#### Private Memory
- **Nexus:** 27.23 MB (stable)
- **rdock:** 7.04 MB (avg, range: 6.54-7.17 MB)
- **Difference:** rdock uses **74.1% less** 🎯

*Private memory represents the actual RAM committed exclusively to the process, making this the most important metric for memory efficiency.*

### System Resources

#### Threads
- **Nexus:** 8 threads
- **rdock:** 3 threads
- **Impact:** Simpler threading model, less context switching overhead

#### Handles
- **Nexus:** 747 handles
- **rdock:** 207 handles
- **Impact:** More efficient resource management, less kernel object overhead

### Binary Size
- **Nexus:** 18.34 MB
- **rdock:** 2.92 MB
- **Difference:** rdock is **84.1% smaller** 🎯

### CPU Time (Accumulated)
- **Nexus:** 164.40s average (process has been running longer)
- **rdock:** 29.84s average (newer process)

*Note: CPU time represents total accumulated time since process start, not current usage rate. Both applications showed stable, idle behavior during testing.*

## Architecture Advantages

### rdock Benefits
1. **Rust Memory Safety:** Zero-cost abstractions with compile-time guarantees
2. **Minimal Dependencies:** Only essential libraries included
3. **Native Windows Integration:** Direct Win32 API usage without abstraction layers
4. **Optimized Build:** Release profile with LTO, strip, and codegen-units=1
5. **Lightweight Threading:** Event-driven architecture with minimal threads

### Comparison Context
- Both docks provide similar visual functionality (icon display, click handling, auto-hide)
- Nexus Dock is a mature product with extensive features and customization options
- rdock is a focused, lightweight alternative optimized for performance

## Configuration

Both docks were tested with similar setups:
- 7 application icons configured
- Auto-hide enabled
- Similar visual styling (background opacity, corner radius)
- Running on Windows 11 with identical system conditions

## Profiling Method

The `profile_docks.ps1` script measures:
- **CPU Time:** Total processor time consumed
- **Working Set:** Physical RAM in use
- **Private Memory:** Committed memory exclusive to the process
- **Threads:** Number of execution threads
- **Handles:** Windows kernel objects (windows, files, registry keys, etc.)

Both processes were monitored in their idle state to measure baseline resource usage, which represents the constant overhead of running each dock.

## Conclusion

rdock achieves its core functionality with **dramatically lower resource consumption** than Nexus Dock. The most significant improvements are:

1. **74% reduction in private memory** - Critical for system performance
2. **84% smaller binary** - Faster loading, less disk space
3. **72% fewer handles** - Reduced kernel overhead
4. **Simpler threading model** - Less complexity and context switching

These improvements make rdock an excellent choice for users prioritizing system resource efficiency while maintaining a functional application dock.

---

*Run `.\profile_docks.ps1` to reproduce these results.*  
*Use `.\profile_docks.ps1 -DurationSeconds 60 -SampleIntervalMs 1000` for longer/different sampling.*
