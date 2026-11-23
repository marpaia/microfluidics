"""Which example can be made at which fab: each passes at every process it names."""

from pathlib import Path

import pytest
from conftest import DESIGNS, copied, design

from microfluidics.project import Project

FLOWJEM, COSINC = "flowjem-su8-pdms", "cosinc-su8-pdms"

MADE_AT = {
    "test_channel": (FLOWJEM, COSINC),
    "mixer": (FLOWJEM, COSINC),
    "goenner_mixer": (FLOWJEM, COSINC),
    "mother_machine": (COSINC,),
}


def test_every_example_says_where_it_is_made() -> None:
    assert sorted(MADE_AT) == sorted(file.stem for file in DESIGNS.glob("*.py"))


@pytest.mark.parametrize("name", sorted(MADE_AT))
def test_an_example_passes_at_every_fab_it_names(name: str) -> None:
    assert tuple(process.name for process in design(name).processes) == MADE_AT[name]
    report = Project(DESIGNS / f"{name}.py").verify()
    assert report.passes and report.processes == MADE_AT[name], [check for check in report.checks if not check.passes]
    for process in MADE_AT[name]:
        for id in ["rules", "mask-rules", "gds-readback"]:
            check = report.check(id, process)
            assert check is not None and check.status == "satisfied", (process, id)


def test_the_mother_machine_is_refused_by_the_mail_order_fab_for_its_trenches(tmp_path: Path) -> None:
    written = (DESIGNS / "mother_machine.py").read_text()
    both = written.replace("import COSINC_SU8_PDMS", "import COSINC_SU8_PDMS, FLOWJEM_SU8_PDMS")
    both = both.replace("processes = (COSINC_SU8_PDMS,)", "processes = (FLOWJEM_SU8_PDMS, COSINC_SU8_PDMS)")
    assert both.count("FLOWJEM_SU8_PDMS") == 2
    report = copied(tmp_path, "mother_machine", text=both).verify()
    assert not report.passes
    refused = report.check("rules", FLOWJEM)
    assert refused is not None and refused.status == "violated"
    # Its trenches are narrower than FlowJEM makes, and its trench layer shallower.
    broken = {finding.rule for finding in refused.findings}
    assert {"feature.width", "feature.space", "layer.height"} <= broken, broken
    narrow = next(finding for finding in refused.findings if finding.rule == "feature.width")
    assert "the process makes nothing under 5.000 um" in narrow.message
    # At the fab it is declared for, the same chip is within every rule.
    held = report.check("rules", COSINC)
    assert held is not None and held.status == "satisfied"
