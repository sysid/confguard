import pytest
from tomlkit import table

from confguard.environment import config


@pytest.mark.skip(reason="requires manual reset of .confguard, 2x run")
def test_sentinel():
    c = config
    assert c.sentinel is None



