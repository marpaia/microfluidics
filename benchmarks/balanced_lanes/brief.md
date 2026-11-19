# Balanced lanes

Design a two-layer mother machine in which one inlet feeds eight lanes of growth trenches and one outlet gathers them, so that every lane gets the same flow.

- **Balance:** every lane's flow within 5 % of the mean of all eight.
- **Shear:** no more than 1 Pa at the wall of any lane, so that cells at the trench mouths stay put.

## What a design must have

- `self.inlet` and `self.outlet`: each an `AccessPad`.
- `self.lane`: a list of eight `TrenchLane`.
- Layers `TRENCH` and `FLOW`.
- Only the regions this benchmark allows.

The benchmark runs the chip with the inlet at 10 mbar, the outlet open, and a growth medium's viscosity at 37 °C (0.7 mPa·s). What a design says of how it is run, and its own requirements, are set aside.
