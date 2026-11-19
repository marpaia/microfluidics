"""A mark the second mask is aligned to the first by."""

import microfluidics as mf
from microfluidics.layers import FLOW, TRENCH
from microfluidics.regions.fit import SLACK
from microfluidics.units import Length, um


class AlignmentMark(mf.Region):
    """A cross on the trench layer framed by four squares on the flow layer. The second mask is
    aligned by centring its squares on the first layer's cross."""

    arm_length: mf.Param[Length] = mf.param(default=300 * um, low=50 * um, high=1000 * um)
    arm_width: mf.Param[Length] = mf.param(default=10 * um, low=5 * um, high=100 * um)
    """Narrow enough that the shallow layer's roof holds."""
    gap: mf.Param[Length] = mf.param(default=10 * um, low=2 * um, high=100 * um)
    """The clearance between the cross and each square: the largest misalignment the mark shows."""

    def build(self) -> None:
        thin = self.arm_width / 2
        near, far = thin + self.gap, self.arm_length / 2
        mf.ensure(far - near >= self.arm_width - SLACK, "the cross is too short for squares to frame it")

        self.cross_upright = self.rectangle(TRENCH, (-thin, -far), (thin, far))
        self.cross_left = self.rectangle(TRENCH, (-far, -thin), (-thin, thin))
        self.cross_right = self.rectangle(TRENCH, (thin, -thin), (far, thin))

        self.square_ne = self.rectangle(FLOW, (near, near), (far, far))
        self.square_nw = self.rectangle(FLOW, (-far, near), (-near, far))
        self.square_se = self.rectangle(FLOW, (near, -far), (far, -near))
        self.square_sw = self.rectangle(FLOW, (-far, -far), (-near, -near))
