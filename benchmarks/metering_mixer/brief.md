# Metering mixer

Design a one-layer chip that meters three liquids into one stream in fixed shares, with every inlet held at the same pressure, so that the shares follow from the channels alone.

- **Buffer:** 89.2 % of what leaves.
- **Reagent:** 10.72 %.
- **Sample:** 0.08 %.

Each share must be within 1 % of its own value: the sample between 0.0792 % and 0.0808 %.

These are the proportions of the calcium assay in Goenner et al., *Scientific Reports* (2025), whose automated flow reported mixing errors of 0.17 %, 1.21 % and 8.08 % for the three after routing.

## What a design must have

- `self.buffer_in`, `self.reagent_in` and `self.sample_in`: each an `AccessPad`, where the three liquids enter.
- `self.outlet`: an `AccessPad`, where the mixture leaves.
- One layer, `FLOW`.
- Only the regions this benchmark allows.

The benchmark runs the chip with every inlet at 5 mbar, the outlet open, and water's viscosity. What a design says of how it is run, and its own requirements, are set aside.
