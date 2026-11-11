@service
class MenuService:
    expanded: bool = False

    def toggle(self) -> None:
        self.expanded = not self.expanded
