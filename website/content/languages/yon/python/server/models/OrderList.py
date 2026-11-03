from dataclasses import dataclass

from models import Order


@dataclass
class OrderList:
    orders: list[Order]
