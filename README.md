# Microfluidics

`microfluidics` is a design tool for microfluidic chips made by soft lithography. A chip is written as a typed Python class describing parametric regions, how the chip is run, what it must do, etc. The Rust kernel checks the design against a fabrication process, solves its flow, tunes its parameters until the requirements hold, checks it across process variation, and writes photomasks with a verification report.

Consider the following developer experience with the Visual Studio Code extension:

<p align="center">
  <img src="docs/assets/screenshot-mixer-3d-speed.png" alt="The mixer design in VS Code: its Python source beside a 3D chip coloured by flow speed, with passing verification checks" width="100%">
</p>
