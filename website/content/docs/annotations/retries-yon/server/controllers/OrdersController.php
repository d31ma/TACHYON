<?php
use Models\OrderReport;

#[RequestRoute("/orders")]
#[Public]
class OrdersController {
    #[Needs]
    public OrdersService $ordersService;

    #[GetRoute]
    public function getOrders(): OrderReport {
        $id = 'A-1';
        $submitted = $this->ordersService->submit($id);
        $report = $this->ordersService->report($id);
        return new OrderReport($submitted, $report);
    }
}
