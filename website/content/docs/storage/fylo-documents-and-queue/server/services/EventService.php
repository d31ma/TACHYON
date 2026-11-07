<?php
#[Service]
class EventService {
  private const string COLLECTION = 'hd-events';
  private const array DEPLOYS = ['kind' => 'deploy'];
  private const array CHANGE = ['count' => 7];
  private const int PAGE_SIZE = 2;
  private const string TOPIC = 'app.render';
  private const string GROUP = 'renderers';

  #[Needs]
  private DataSource $dataSource;

  public function report(): array {
    $this->dataSource->createCollection(self::COLLECTION);
    $id = $this->dataSource->insert(self::COLLECTION, self::DEPLOYS)['id'];
    $this->dataSource->insert(self::COLLECTION, self::DEPLOYS);
    $this->dataSource->insert(self::COLLECTION, self::DEPLOYS);
    $found = $this->dataSource->findById(self::COLLECTION, $id);
    $patched = $this->dataSource->patch(self::COLLECTION, $id, self::CHANGE);
    $scrolled = $this->countPages();
    $job = $this->runJob();
    $this->dataSource->deleteById(self::COLLECTION, $id);
    $gone = $this->dataSource->findById(self::COLLECTION, $id) === null;
    $dropped = $this->dataSource->dropCollection(self::COLLECTION);
    return ['found' => $found['kind'], 'patched' => $patched['count'], 'job' => $job, 'gone' => $gone, 'dropped' => $dropped] + $scrolled;
  }

  private function countPages(): array {
    $pages = 0;
    $seen = 0;
    $cursor = null;
    $more = true;
    while ($more) {
      $window = $this->dataSource->scroll(self::COLLECTION, self::DEPLOYS, $cursor, self::PAGE_SIZE);
      $pages += 1;
      $seen += count($window['content']);
      $cursor = $window['nextCursor'];
      $more = $cursor !== null;
    }
    return ['pages' => $pages, 'seen' => $seen];
  }

  private function runJob(): array {
    $this->dataSource->publish(self::TOPIC, ['app' => 'web'], 'render:web');
    $delivery = $this->dataSource->claim(self::TOPIC, self::GROUP, 1, 30000)[0];
    $this->dataSource->ack(self::TOPIC, self::GROUP, $delivery);
    return $delivery['payload'];
  }
}
