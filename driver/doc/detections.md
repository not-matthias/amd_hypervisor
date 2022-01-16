# Hypervisor Detections

## TSC Offset

The value of `rdtsc` is calculated like this:
- `TSCFreq = Core P0 frequency * TSCRatio, so TSCRatio = (Desired TSCFreq) / Core P0 frequency.`
- `TSC Value (in guest) = (P0 frequency * TSCRatio * t) + VMCB.TSC_OFFSET + (Last Value Written to TSC) * TSCRatio
Where t is time since the TSC was last written via the TSC MSR (or since reset if not written)`

There's 2 ways we could implement this: 
- Write to `TSC_RATIO` msr
- Adjust `TSC_OFFSET` in `VMCB`
