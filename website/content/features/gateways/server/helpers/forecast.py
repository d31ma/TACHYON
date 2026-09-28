import json
import sys

call = json.load(sys.stdin)
months = call["arguments"][0]
print(json.dumps({"method": call["method"], "months": months, "growth": [1.5] * months}))
