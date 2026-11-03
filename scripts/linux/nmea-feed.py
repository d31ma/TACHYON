import functools
import operator
import os
import socket
import sys
import time


class NmeaFeed:
    LATITUDE = "5130.0000,N"
    LONGITUDE = "00007.5000,W"

    @staticmethod
    def sentence(body):
        check = functools.reduce(operator.xor, body.encode(), 0)
        return "$%s*%02X\r\n" % (body, check)

    @classmethod
    def fixes(cls):
        now = time.gmtime()
        clock = time.strftime("%H%M%S", now)
        day = time.strftime("%d%m%y", now)
        position = "%s,%s" % (cls.LATITUDE, cls.LONGITUDE)
        return (cls.sentence("GPGGA,%s.00,%s,1,08,0.9,10.0,M,0.0,M,," % (clock, position))
                + cls.sentence("GPRMC,%s.00,A,%s,0.0,0.0,%s,,," % (clock, position, day)))

    @classmethod
    def serve(cls, connection):
        try:
            while True:
                connection.sendall(cls.fixes().encode())
                time.sleep(1)
        except OSError:
            os._exit(0)

    @classmethod
    def main(cls, path):
        server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        server.bind(path)
        server.listen(4)
        while True:
            connection, _ = server.accept()
            if os.fork() == 0:
                cls.serve(connection)
            connection.close()


NmeaFeed.main(sys.argv[1])
