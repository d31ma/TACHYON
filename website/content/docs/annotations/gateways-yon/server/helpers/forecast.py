import json
import sys

call = json.load(sys.stdin)
months = call["arguments"][0]
print(json.dumps({"months": months, "growth": [1.5] * months}))
