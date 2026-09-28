<?php
#[HTTPClient]
class CatalogClient {
    public BehaviorContext $behaviors;

    #[GetRequest("/products")]
    public function all(#[RequestParam("page")] int $page): array {}

    #[PostRequest("/products")]
    public function add(#[RequestBody] array $product): array {}

    #[GetRequest("/products")]
    public function first(): array { return ['query' => ['page' => 1]]; }

    #[OnStart]
    public function initialize(): void {
        $this->behaviors = Behaviors::create(CatalogSetup::class, [
            'partition' => 'catalog',
            'signal' => new BehaviorSignal(),
        ]);
    }
}
