<?php
#[Description("Behavior conformance Backend")]
class Backend {
    public static array $entries = [];
    public static int $calls = 0;
    public static int $exchanges = 0;
    public static int $authorizations = 0;

    public static function authorize($call, $execution) {
        self::$authorizations++;
        if ($execution->partition !== 'catalog') return false;
        if ($call['method'] === 'remote') return true;
        return $call['keyArguments'][0]['size'] === 5;
    }

    public static function get($key, $execution) {
        return self::$entries[$key] ?? ['hit' => false];
    }

    public static function set($key, $value, $execution) {
        self::$entries[$key] = ['hit' => true, 'value' => $value, 'expiresAt' => $execution->expiresAt];
    }

    public static function delete($key, $execution) {
        unset(self::$entries[$key]);
    }

    public static function exchange($request, $execution) {
        self::$exchanges++;
        if ($request['redirect'] !== 'error') throw new Exception('transport policy');
        if ($request['credentials'] !== 'omit') throw new Exception('transport policy');
        if ($request['maxResponseBytes'] !== 1048576) throw new Exception('transport policy');
        return ['path' => $request['path']];
    }

    public static function execute($command, $execution) {
        if ($command['operation'] !== 'query') return $command;
        $query = $command['query'];
        if ($query['where'][0]['value'] !== 'active') throw new Exception('query binding');
        if ($query['kind'] === 'count') return 1;
        if ($query['kind'] === 'exists') return true;
        return [['id' => 'one', 'status' => 'active']];
    }

    public static function authorizeEntity($command, $execution) {
        return $command['entity'] === 'ProductEntity' && $execution->partition === 'catalog';
    }
}

