"""A mark the second mask is aligned to the first by."""

import microfluidics as mf
from microfluidics.layers import FLOW, TRENCH
from microfluidics.regions.fit import SLACK
from microfluidics.units import Length, um


class AlignmentMark(mf.Region):
    """A cross on the trench layer inside a square window on the flow layer. The flow mask is
    clear only inside its features, so the cross made by the first mask is seen through the
    window, and the second mask is aligned by centring the window on it."""

    arm_length: mf.Param[Length] = mf.param(default=150 * um, low=50 * um, high=1000 * um)
    arm_width: mf.Param[Length] = mf.param(default=10 * um, low=5 * um, high=100 * um)
    gap: mf.Param[Length] = mf.param(default=10 * um, low=2 * um, high=100 * um)
    """The clearance between the tip of each arm and the window's edge: the largest misalignment
    through which the whole cross is still seen."""

    def build(self) -> None:
        thin, far = self.arm_width / 2, self.arm_length / 2
        reach = far + self.gap
        mf.ensure(far - thin >= self.arm_width - SLACK, "the cross's arms are shorter than they are wide")

        self.cross_upright = self.rectangle(TRENCH, (-thin, -far), (thin, far))
        self.cross_left = self.rectangle(TRENCH, (-far, -thin), (-thin, thin))
        self.cross_right = self.rectangle(TRENCH, (thin, -thin), (far, thin))
        # The window is narrow enough for its roof to hold over the flow layer.
        self.window = self.rectangle(FLOW, (-reach, -reach), (reach, reach))

        self.fiducial(at=(0 * um, 0 * um))
