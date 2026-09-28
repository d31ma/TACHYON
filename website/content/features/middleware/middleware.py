import json
import urllib.parse
from http import HTTPStatus


class Middleware:
    @staticmethod
    def before(request):
        path = urllib.parse.urlsplit(request.full_url).path
        if path.startswith("/private") and request.get_header("Authorization") != "let-me-in":
            body = json.dumps({"error": "Unauthorized"})
            return (HTTPStatus.UNAUTHORIZED, {"content-type": "application/json"}, body)
        return None

    @staticmethod
    def after(request):
        return (HTTPStatus.NO_CONTENT, {"x-served-by": "tachyon"}, "")
