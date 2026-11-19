"""Three liquids metered into one stream."""

import microfluidics as mf
from microfluidics.layers import FLOW
from microfluidics.processes import CONSERVATIVE_SU8_PDMS
from microfluidics.regions import AccessPad, Junction, LineMonitor, Serpentine
from microfluidics.units import Length, mbar, mm, mPa_s, pct, um


class GoennerMixer(mf.Design):
    """Three inputs metered into one stream: the length of each one's way to a junction sets its
    share, and a last serpentine gives the stream time to mix.

    Every channel from a pad to the junction's outlet has one width in one layer, so a layer made
    thick or thin, or features made wide or narrow, change all three resistances alike and leave
    the shares as they are: the shares follow from the lengths, which a mask holds closely."""

    process = CONSERVATIVE_SU8_PDMS
    slab = mf.Slab(30 * mm, 22 * mm, thickness=4 * mm)
    glass = mf.Glass(75 * mm, 25 * mm, thickness=1 * mm)
    layers = mf.Stack({FLOW: mf.LayerSpec(height=50 * um, gds=1)})

    #: The one width of every metering channel.
    width: mf.Param[Length] = mf.param(default=30 * um)

    def build(self) -> None:
        width, clear = self.width, 600 * um
        pitch, runs = 80 * um, 145
        # The sample's way is some thousand times the buffer's. Its serpentine stands on its pad
        # and folds away to the left, leaving at its far upper corner; the junction's first input
        # is straight above that corner.
        right = 16 * mm
        left = right - pitch * (runs - 1)
        self.meet = Junction(inputs=3, pitch=2 * mm, collector_width=width, input_width=width, stub_length=300 * um).at(
            left, 16.7 * mm
        )
        self.sample_in = AccessPad(channel_width=width).at(right, 1.55 * mm)
        self.sample_resistor = Serpentine(runs=runs, run_length=5.6 * mm, width=width, pitch=pitch).attach(
            lambda resistor: resistor.inlet, to=self.sample_in.opening, offset=clear
        )
        self.sample_lead = self.route(FLOW, width, self.sample_in.opening, self.sample_resistor.inlet)
        self.sample_feed = self.route(FLOW, width, self.sample_resistor.outlet, self.meet.input[0])

        self.reagent_in = AccessPad(channel_width=width).at(left + 2 * mm + pitch * 2, 12 * mm)
        self.reagent_resistor = Serpentine(runs=3, run_length=1 * mm, width=width, pitch=pitch).attach(
            lambda resistor: resistor.inlet, to=self.reagent_in.opening, offset=clear
        )
        self.reagent_lead = self.route(FLOW, width, self.reagent_in.opening, self.reagent_resistor.inlet)
        self.reagent_feed = self.route(FLOW, width, self.reagent_resistor.outlet, self.meet.input[1])

        # The buffer's way is the short one: its pad stands just under the junction, at the input
        # nearest the junction's outlet.
        self.buffer_in = AccessPad(channel_width=width).at(left + 4 * mm, 14.8 * mm)
        self.buffer_feed = self.route(FLOW, width, self.buffer_in.opening, self.meet.input[2])

        self.mixer = Serpentine(runs=5, run_length=3 * mm, width=200 * um, pitch=800 * um).attach(
            lambda mixer: mixer.inlet, to=self.meet.outlet
        )
        self.outlet = AccessPad(channel_width=200 * um).attach(
            lambda pad: pad.opening, to=self.mixer.outlet, offset=clear
        )
        self.exit = self.route(FLOW, 200 * um, self.mixer.outlet, self.outlet.opening)
        # Measured on the made master to learn what the process did.
        self.monitor = LineMonitor(first=30 * um).at(1.5 * mm, 10 * mm)

        self.buffer, self.reagent, self.sample = mf.Species(), mf.Species(), mf.Species()
        # Every inlet at one pressure, as from one source: the shares follow from the resistances
        # alone.
        self.operate(
            viscosity=1 * mPa_s,
            drive={
                self.buffer_in: 5 * mbar,
                self.reagent_in: 5 * mbar,
                self.sample_in: 5 * mbar,
                self.outlet: 0 * mbar,
            },
            feed={
                self.buffer_in: {self.buffer: 100 * pct},
                self.reagent_in: {self.reagent: 100 * pct},
                self.sample_in: {self.sample: 100 * pct},
            },
        )
        self.buffer_share = mf.Requirement(
            "Buffer is 89.2 % of the mixture",
            mf.fraction(self.buffer, self.outlet),
            mf.target(89.2 * pct, within=1 * pct, relative=True),
        )
        self.reagent_share = mf.Requirement(
            "Reagent is 10.72 % of the mixture",
            mf.fraction(self.reagent, self.outlet),
            mf.target(10.72 * pct, within=1 * pct, relative=True),
        )
        self.sample_share = mf.Requirement(
            "Sample is 0.08 % of the mixture",
            mf.fraction(self.sample, self.outlet),
            mf.target(0.08 * pct, within=1 * pct, relative=True),
        )
        self.tune(self.reagent_resistor, Serpentine.run_length)
        self.tune(self.sample_resistor, Serpentine.run_length)
