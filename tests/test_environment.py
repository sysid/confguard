from confguard.environment import config


def test_environment():
    c = config
    c.load_config()
    _ = None
