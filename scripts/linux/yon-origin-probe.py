#!/usr/bin/env python3

import importlib.util
import json
import sys
from pathlib import Path


class YonOriginProbe:
    WAIT_SECONDS = 30

    @staticmethod
    def load_accessibility_probe():
        path = Path(__file__).with_name("a11y-probe.py")
        specification = importlib.util.spec_from_file_location("accessibility_probe", path)
        module = importlib.util.module_from_spec(specification)
        specification.loader.exec_module(module)
        return module

    @staticmethod
    def main(steps):
        probe = YonOriginProbe.load_accessibility_probe()
        probe.Atspi.init()
        actions = {
            "wait": lambda texts: probe.wait_for_controls(texts, YonOriginProbe.WAIT_SECONDS),
            "click": probe.activate_control,
        }
        for kind, value in steps:
            actions[kind](value)
        print("PASS: the Linux app showed every Yon answer", flush=True)


if __name__ == "__main__":
    YonOriginProbe.main(json.loads(sys.argv[1]))
