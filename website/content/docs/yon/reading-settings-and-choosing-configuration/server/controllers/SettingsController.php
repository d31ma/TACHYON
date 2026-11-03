<?php
use Models\Settings;

#[RequestRoute("/settings")]
#[Public]
class SettingsController {
    #[GetRoute]
    public function getSettings(): Settings {
        $application = json_decode(getenv('YON_APPLICATION_JSON'), true, 512, JSON_THROW_ON_ERROR);
        $catalog = Behaviors::properties(CatalogSetup::class, $application);
        $endpoint = Behaviors::values(Endpoint::class, $application);
        $profile = $endpoint['profile'];
        $signal = new BehaviorSignal();
        $context = Behaviors::create(CatalogSetup::class, [
            'partition' => 'public-catalog',
            'signal' => $signal,
            'profiles' => [$profile],
            'properties' => $application,
        ]);
        $context->dispose();
        $signal->abort();
        return new Settings($profile, $catalog['pageSize']);
    }
}
