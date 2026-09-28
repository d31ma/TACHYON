<?php
use Models\Configuration;

#[RequestRoute("/configuration")]
#[Public]
class ConfigurationController {
    #[GetRoute]
    public function getConfiguration(): Configuration {
        $application = json_decode(getenv('YON_APPLICATION_JSON'), true, 512, JSON_THROW_ON_ERROR);
        $settings = Behaviors::properties(CatalogSettings::class, $application);
        $endpoint = Behaviors::values(Endpoint::class, $application);
        return new Configuration($settings['pageSize'], $endpoint['port']);
    }
}
