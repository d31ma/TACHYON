from typing import Annotated

from models import Problem


@service
class OrderService:
    order_client: Annotated[OrderClient, needs]
    status: int = 0
    title: str = ''
    mistakes: int = 0

    async def send(self) -> None:
        failure = await self._failure()
        self._show(BehaviorError.status_of(failure), self._problem(failure))

    async def _failure(self) -> BehaviorError:
        try:
            await self.order_client.send({'name': '', 'level': 12})
        except BehaviorError as error:
            return error
        return BehaviorError('none', 'no failure', None)

    def _problem(self, failure: BehaviorError) -> Problem:
        problem = BehaviorError.body_of(failure, Problem)
        return problem if problem is not None else Problem('', 0, [])

    def _show(self, status: int, problem: Problem) -> None:
        self.status = status
        self.title = problem.title
        self.mistakes = len(problem.errors)
