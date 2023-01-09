from confguard.adapter import TomlRepoConfGuard
from confguard.environment import CONFGUARD_CONFIG_FILE
from confguard.model import ConfGuard
from tests.conftest import REF_PROJ, SENTINEL, TEST_PROJ


class TestTomlRepoConfGuard:
    def test_get(self):
        repo = TomlRepoConfGuard(source_dir=TEST_PROJ)
        cg = repo.get()

        assert isinstance(cg, ConfGuard)
        assert cg.targets == [".envrc", ".run", "xxx/xxx.txt"]

    def test_get_with_files(self):
        repo = TomlRepoConfGuard(source_dir=TEST_PROJ / "..")
        cg = repo.get()

        assert isinstance(cg, ConfGuard)
        assert cg.files == [".envrc", ".run", "xxx/xxx.txt"]

    def test_add_without_change(self):
        repo = TomlRepoConfGuard(source_dir=TEST_PROJ)
        cg = ConfGuard(source_dir=TEST_PROJ, targets=[".envrc", ".run", "xxx/xxx.txt"])
        repo.add(cg)
        under_test = (TEST_PROJ / CONFGUARD_CONFIG_FILE).read_text()
        ref = (REF_PROJ / CONFGUARD_CONFIG_FILE).read_text()
        assert under_test == ref

    def test_add_new(self):
        repo = TomlRepoConfGuard(source_dir=TEST_PROJ)
        cg = ConfGuard(source_dir=TEST_PROJ, targets=[".envrc", ".run", "xxx/xxx.txt"])
        cg.create_sentinel()
        repo.add(cg)
        under_test = (TEST_PROJ / CONFGUARD_CONFIG_FILE).read_text()
        ref = (REF_PROJ / CONFGUARD_CONFIG_FILE).read_text()
        assert under_test != ref
        assert "[_internal_] # DO NOT EDIT FROM HERE" in under_test

    def test_add_update(self):
        repo = TomlRepoConfGuard(source_dir=TEST_PROJ)
        cg = ConfGuard(source_dir=TEST_PROJ, targets=[".envrc", ".run", "xxx/xxx.txt"])
        cg.create_sentinel()
        repo.add(cg)

        # when: new data
        cg.targets = [".envrc", ".run", "xxx/xxx.txt", "xxx/xxx2.txt"]
        cg.sentinel = SENTINEL
        repo.add(cg)

        # then: updated
        under_test = (TEST_PROJ / CONFGUARD_CONFIG_FILE).read_text()
        assert "[_internal_] # DO NOT EDIT FROM HERE" in under_test
        assert SENTINEL in under_test

    def test_add_removed_sentinel(self):
        repo = TomlRepoConfGuard(source_dir=TEST_PROJ)
        cg = ConfGuard(source_dir=TEST_PROJ, targets=[".envrc", ".run", "xxx/xxx.txt"])
        cg.create_sentinel()
        repo.add(cg)
        under_test = (TEST_PROJ / CONFGUARD_CONFIG_FILE).read_text()
        assert "[_internal_] # DO NOT EDIT FROM HERE" in under_test

        cg.sentinel = None

        repo.add(cg)
        under_test = (TEST_PROJ / CONFGUARD_CONFIG_FILE).read_text()
        ref = (REF_PROJ / CONFGUARD_CONFIG_FILE).read_text()
        assert under_test == ref
        assert "[_internal_] # DO NOT EDIT FROM HERE" not in under_test
