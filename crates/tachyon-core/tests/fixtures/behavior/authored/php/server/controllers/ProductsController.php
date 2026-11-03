<?php
#[RequestRoute("/products")]
#[Public]
class ProductsController {
    #[GetRoute]
    public function getProducts() {
        $signal = new BehaviorSignal();
        $values = ['YON_CATALOG_SIZE' => '25', 'YON_CATALOG_ENABLED' => 'true'];
        HandlerChecks::checkConfiguration($values);
        HandlerChecks::checkValueLimits();
        HandlerChecks::checkValidation();
        HandlerChecks::checkInactiveConfiguration($signal);
        $context = Behaviors::create(AdaptersSetup::class, ['partition' => 'catalog', 'signal' => $signal, 'profiles' => ['test'], 'properties' => $values]);
        $result = HandlerChecks::exercise($context, $signal);
        header('Content-Type: application/json');
        echo json_encode($result, JSON_THROW_ON_ERROR);
        $context->dispose();
    }
}
