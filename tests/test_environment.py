import pytest
from tomlkit import table

from confguard.environment import config


@pytest.mark.skip(reason="requires manual reset of .confguard, 2x run")
def test_sentinel():
    c = config
    assert c.sentinel is None


def test_add_sentinel():
    c = config
    c.confguard_add_sentinel("test-12345")
    c.load_confguard()
    assert c.confguard["_internal_"]["sentinel"] == "test-12345"
    assert c.sentinel == "test-12345"


def test_update_sentinel():
    # given
    c = config
    c.confguard_add_sentinel("test-12345")

    # when
    c.confguard_update_sentinel("test-99999")
    c.load_confguard()

    # then
    assert c.confguard["_internal_"]["sentinel"] == "test-99999"
    assert c.sentinel == "test-99999"


def test_remove_sentinel():
    c = config
    c.confguard_add_sentinel("test-12345")
    c.confguard_remove_sentinel()
    c.load_confguard()

    assert c.sentinel is None
