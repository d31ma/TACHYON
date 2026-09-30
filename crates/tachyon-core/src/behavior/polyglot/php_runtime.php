final class BehaviorError extends \RuntimeException {
    public function __construct(public readonly string $behaviorCode, string $message, public readonly ?string $path=null) { parent::__construct($message); }
}
final class BehaviorSignal {
    private bool $cancelled = false;
    public function abort(): void { $this->cancelled = true; }
    public function aborted(): bool { return $this->cancelled; }
}
final class BehaviorCancellation {
    public function __construct(private readonly BehaviorSignal $context,private readonly BehaviorSignal $owner,private readonly int $until) {}
    public function aborted(): bool { return $this->context->aborted() || $this->owner->aborted() || hrtime(true)>=$this->until; }
}
final class BehaviorExecution {
    public readonly int $deadline;
    private readonly int $until;
    public readonly BehaviorCancellation $signal;
    private readonly BehaviorSignal $contextSignal;
    public array $policy = [];
    public int $expiresAt = 0;
    public function __construct(public readonly string $partition, public readonly string $environment,
        BehaviorSignal $signal, private readonly BehaviorSignal $owner, int $timeoutMs) {
        $this->deadline = (int)floor(microtime(true)*1000) + $timeoutMs;
        $this->until = hrtime(true) + $timeoutMs*1000000;
        $this->contextSignal=$signal;
        $this->signal=new BehaviorCancellation($signal,$owner,$this->until);
    }
    public function check(): void {
        if ($this->contextSignal->aborted() || $this->owner->aborted()) throw new BehaviorError('cancelled','Behavior operation was cancelled.');
        if (hrtime(true) >= $this->until) throw new BehaviorError('timeout','Behavior operation exceeded its deadline.');
    }
}
final class BehaviorBoundary {
    public static function invalid(string $message): never { throw new BehaviorError('invalid',$message); }
    public static function gateway(string $key, array $arguments): mixed {
        if (!class_exists('YonGateway', false)) throw new BehaviorError('adapter','Gateway call failed.');
        $answer = YonGateway::call($key,$arguments);
        if (is_array($answer) && array_key_exists('result',$answer)) return $answer['result'];
        $kind = is_array($answer) ? ($answer['error'] ?? 'adapter') : 'adapter';
        if (!in_array($kind,['invalid','timeout','capacity','adapter'],true)) $kind = 'adapter';
        throw new BehaviorError($kind,'Gateway call failed.');
    }
    public static function keys(mixed $value, array $allowed): void {
        if (!is_array($value) || array_diff(array_keys($value),$allowed)) self::invalid('Unknown or invalid behavior options.');
    }
    public static function copy(mixed $value, int $limit=1048576): mixed {
        $nodes=0; $characters=0;
        $result=self::walk($value,0,$nodes,$characters,$limit);
        if (strlen(self::encode($result)) > $limit) self::invalid('Behavior JSON exceeds byte bounds.');
        return $result;
    }
    public static function encode(mixed $value): string {
        try { return json_encode($value,JSON_THROW_ON_ERROR|JSON_UNESCAPED_UNICODE|JSON_UNESCAPED_SLASHES); }
        catch (\Throwable) { self::invalid('Behavior JSON is not valid UTF-8 JSON.'); }
    }
    private static function walk(mixed $value,int $depth,int &$nodes,int &$characters,int $limit): mixed {
        if (++$nodes>10000 || $depth>32) self::invalid('Behavior JSON exceeds structural bounds.');
        if ($value===null || is_bool($value)) return $value;
        if (is_int($value) && abs($value)<=9007199254740991) return $value;
        if (is_float($value) && is_finite($value)) return $value;
        if (is_string($value)) { $characters+=strlen($value); if($characters>$limit)self::invalid('Behavior JSON exceeds byte bounds.');return $value; }
        $object=$value instanceof \stdClass;
        if (!is_array($value) && !$object) self::invalid('Behavior values must be finite JSON data.');
        $items=$object?get_object_vars($value):$value;
        if(count($items)>10000)self::invalid('Behavior collection exceeds bounds.');
        $list=!$object && array_is_list($items);
        if(!$list)ksort($items,SORT_STRING);
        $result=[];
        foreach($items as $key=>$child) {
            if(!$list) {
                if(!is_string($key) || (in_array($key,['__proto__','prototype','constructor','behaviors'],true)||($key==='tac'&&$depth!==0))) self::invalid('Unsafe behavior JSON field.');
                self::walk($key,$depth+1,$nodes,$characters,$limit);
            }
            $result[$key]=self::walk($child,$depth+1,$nodes,$characters,$limit);
        }
        return $object?(object)$result:$result;
    }
    public static function page(array $value,array $policy): array {
        self::keys($value,['page','size','offset','sort']);self::copy($value,4096);
        $page=array_key_exists('page',$value)?$value['page']:0;$size=array_key_exists('size',$value)?$value['size']:$policy['defaultSize'];
        if(!is_int($page)||!is_int($size)||$page<0||$page>1000000||$size<1||$size>$policy['maxSize']||$page*$size>1000000)self::invalid('Invalid behavior pagination bounds.');
        if(array_key_exists('offset',$value)&&$value['offset']!==$page*$size)self::invalid('Invalid behavior pagination offset.');
        $sort=$value['sort']??[];$seen=[];$normalized=[];
        if(!is_array($sort)||!array_is_list($sort)||count($sort)>16)self::invalid('Invalid behavior sort.');
        foreach($sort as $item) {
            if(!is_string($item))self::invalid('Invalid behavior sort.');
            $parts=explode(':',$item);$field=$parts[0];$direction=$parts[1]??'asc';
            if(count($parts)>2||!in_array($field,$policy['sort'],true)||!in_array($direction,['asc','desc'],true)||isset($seen[$field]))self::invalid('Invalid behavior sort.');
            $seen[$field]=true;$normalized[]="$field:$direction";
        }
        return ['page'=>$page,'size'=>$size,'offset'=>$page*$size,'sort'=>$normalized];
    }
    public static function adapter(callable $function,array $args,BehaviorExecution $execution): mixed {
        $execution->check();
        try {$result=$function(...$args);$execution->check();return $result;}
        catch(BehaviorError $error){throw $error;}
        catch(\Throwable){throw new BehaviorError('adapter','Behavior adapter failed.');}
    }
    public static function mapValue(array $policy,array $args): array {
        $value=$args[$policy['argument']]??null;
        if(!is_array($value)||array_is_list($value))self::invalid('A behavior mapping reads one JSON object argument.');
        $output=[];
        foreach($policy['fields'] as $field) {
            if(!array_key_exists('source',$field)) { $output[$field['target']]=$field['constant']??null; continue; }
            $current=$value;
            foreach(explode('.',$field['source']) as $name)$current=is_array($current)&&array_key_exists($name,$current)?$current[$name]:null;
            $output[$field['target']]=$current;
        }
        return self::copy($output);
    }
    public static function exchangePath(array $policy,array $args): string {
        $path=$policy['path'];
        foreach(($policy['pathVariables']??[]) as $name=>$index) {
            $value=$args[$index]??null;
            $text=is_bool($value)?($value?'true':'false'):((is_int($value)||is_float($value))?(string)$value:$value);
            if(!is_string($text)||!preg_match('/^[A-Za-z0-9._~-]{1,256}$/D',$text)||$text==='.'||$text==='..')self::invalid('A path variable may use only letters, digits and - _ . ~');
            $path=str_replace('{'.$name.'}',$text,$path);
        }
        return $path;
    }
    public static function exchangeRequest(array $policy,array $args): array {
        $pick=function(int $index)use($args) {
            if(!array_key_exists($index,$args))self::invalid('A request argument is missing.');
            return $args[$index];
        };
        $descriptor=['query'=>[],'headers'=>[]];
        foreach(($policy['query']??[]) as $name=>$index)$descriptor['query'][$name]=$pick($index);
        foreach(($policy['headers']??[]) as $name=>$index)$descriptor['headers'][$name]=$pick($index);
        if(array_key_exists('body',$policy))$descriptor['body']=$pick($policy['body']);
        return $descriptor;
    }
    public static function retryable(\Throwable $error): bool {
        return !($error instanceof BehaviorError) || $error->behaviorCode==='adapter';
    }
    public static function backoff(array $policy,int $attempt): int {
        $delay=min($policy['delayMs']*($policy['multiplier']**$attempt),$policy['maxDelayMs']);
        return (int)round($delay+(mt_rand()/mt_getrandmax())*$policy['jitterMs']);
    }
    public static function safePath(string $path): bool {
        if(!str_starts_with($path,'/')||str_starts_with($path,'//')||strlen($path)>2048)return false;
        if(strpbrk($path,"\\?#%")!==false||preg_match('/[\x00-\x20]/',$path))return false;
        foreach(explode('/',$path) as $part)if($part==='.'||$part==='..')return false;
        return true;
    }
    public static function entityId(mixed $id): void {
        if(!((is_string($id)&&$id!==''&&strlen($id)<=256)||(is_int($id)&&abs($id)<=9007199254740991)))self::invalid('Invalid entity identifier.');
        self::copy($id,1024);
    }
}
final class BehaviorOpenTransactions {
    private static array $open=[];
    public static function of(string $adapter): ?string {return self::$open[$adapter]??null;}
    public static function within(string $adapter,string $identifier,callable $body): mixed {
        $outer=self::$open;
        self::$open[$adapter]=$identifier;
        try {return $body();} finally {self::$open=$outer;}
    }
}
final class BehaviorContext {
    private array $adapters=[];
    private array $slots=[];
    private int $active=0;
    private readonly BehaviorSignal $signal;
    private readonly BehaviorSignal $owner;
    private readonly int $timeoutMs;
    public readonly string $partition;
    public function __construct(private readonly array $records,array $options) {
        BehaviorBoundary::keys($options,['partition','signal','timeoutMs','profiles','properties']);
        $partition=$options['partition']??null;$signal=$options['signal']??null;$timeout=$options['timeoutMs']??10000;
        if(!is_string($partition)||trim($partition)===''||strlen($partition)>256||!preg_match('//u',$partition)||!($signal instanceof BehaviorSignal)||!is_int($timeout)||$timeout<1||$timeout>60000)BehaviorBoundary::invalid('Explicit bounded partition, BehaviorSignal and timeout are required.');
        $this->partition=$partition;$this->owner=$signal;$this->signal=new BehaviorSignal();$this->timeoutMs=$timeout;
    }
    public function configure(callable $factory): void {
        $execution=$this->begin();
        try {
            $adapters=BehaviorBoundary::adapter($factory,[$execution],$execution);
            BehaviorBoundary::keys($adapters,['caches','http','entities','authorize','authorizeEntity']);
            foreach(['caches','http','entities'] as $group) {
                $collection=$adapters[$group]??[];
                if(!is_array($collection)||count($collection)>128)BehaviorBoundary::invalid('Invalid adapter collection.');
                foreach(array_keys($collection) as $name)if(!is_string($name)||!preg_match('/^[A-Za-z_$][A-Za-z0-9_$]{0,127}$/D',$name)||in_array($name,['__proto__','prototype','constructor','behaviors','tac'],true))BehaviorBoundary::invalid('Invalid adapter name.');
            }
            if(array_key_exists('authorize',$adapters)&&!is_callable($adapters['authorize']))BehaviorBoundary::invalid('Authorize must be callable.');
            if(array_key_exists('authorizeEntity',$adapters)&&!is_callable($adapters['authorizeEntity']))BehaviorBoundary::invalid('Entity authorize must be callable.');
            $this->adapters=$adapters;
        } finally {$this->active--;}
    }
    private ?string $setupKey=null;
    public function adapters(): array {return $this->adapters;}
    public function share(string $setupKey,array $adapters): void {$this->setupKey=$setupKey;$this->adapters=$adapters;}
    public function dispose(): void {
        $this->signal->abort();
        if($this->setupKey===null)return;
        $setupKey=$this->setupKey;$this->setupKey=null;
        Behaviors::releaseSetup($setupKey);
    }
    private function begin(): BehaviorExecution {
        $execution=new BehaviorExecution($this->partition,'__BEHAVIOR_ENVIRONMENT__',$this->signal,$this->owner,$this->timeoutMs);
        $execution->check();if($this->active>=64)BehaviorBoundary::invalid('Behavior concurrency exceeds bounds.');$this->active++;return $execution;
    }
    private function adapter(string $group,string $name,string $operation): callable {
        $adapter=$this->adapters[$group][$name]??null;
        $function=is_array($adapter)?($adapter[$operation]??null):[$adapter,$operation];
        if(!is_callable($function))BehaviorBoundary::invalid('Required behavior adapter is missing.');return $function;
    }
    private function arguments(array $args,array $metadata): array {
        if(isset($metadata['valid'])) {
            $policy=$metadata['valid'];
            $args[$policy['argument']]=BehaviorValidation::apply($policy['schema'],$args[$policy['argument']]??null);
        }
        if(count($args)>32)BehaviorBoundary::invalid('Too many behavior arguments.');
        if(isset($metadata['pageable'])) {
            $policy=$metadata['pageable'];$index=$policy['argument'];
            if(!array_key_exists($index,$args)||!is_array($args[$index]))BehaviorBoundary::invalid('Pagination requires an argument map.');
            $args[$index]=BehaviorBoundary::page($args[$index],$policy);
        }
        return $args;
    }
    private function record(object $instance,string $type,string $method): array {
        if(get_class($instance)!==$type||!isset($this->records[$type]['methods'][$method])||($instance->behaviors??null)!==$this)BehaviorBoundary::invalid('Behavior instance requires its source-local context.');
        return $this->records[$type];
    }
    private function key(array $record,string $method,array $args,BehaviorExecution $execution,string $operation): string {
        $selected=[];foreach($record['methods'][$method]['cacheable']['keyArgs'] as $index) {
            if(!array_key_exists($index,$args))BehaviorBoundary::invalid('Cache key argument is missing.');$selected[]=$args[$index];
        }
        $selected=BehaviorBoundary::copy($selected,4096);
        $authorize=$this->adapters['authorize']??null;
        $call=['operation'=>$operation,'cache'=>$record['cache']['name'],'type'=>$record['name'],'method'=>$method,'keyArguments'=>BehaviorBoundary::copy($selected)];
        if(!is_callable($authorize)||BehaviorBoundary::adapter($authorize,[$call,$execution],$execution)!==true)throw new BehaviorError('unauthorized','Behavior cache access denied.');
        return BehaviorBoundary::encode(BehaviorBoundary::copy([$this->partition,$record['cache']['name'],'__BEHAVIOR_MODULE_ID__',$record['name'],$method,$selected],4096));
    }
    public function invoke(object $instance,string $type,string $method,array $args,callable $original): mixed {
        $record=$this->record($instance,$type,$method);$metadata=$record['methods'][$method];$execution=$this->begin();
        try {
            $args=$this->arguments($args,$metadata);$key=null;
            if(isset($metadata['query']))return $this->query($metadata['query'],$args,$execution);
            if(isset($metadata['cachePut'])||isset($metadata['cacheEvict']))return $this->mutate($record,$metadata,$args,$original,$execution);
            if(isset($metadata['cacheable'])) {
                $policy=$record['cache'];$key=$this->key($record,$method,$args,$execution,'cache:read');
                $hit=BehaviorBoundary::adapter($this->adapter('caches',$policy['name'],'get'),[$key,$execution],$execution);
                BehaviorBoundary::keys($hit,['hit','value','expiresAt']);
                if(!isset($hit['hit'])||!is_bool($hit['hit']))BehaviorBoundary::invalid('Invalid cache response.');
                if($hit['hit']) {
                    if(!isset($hit['expiresAt'])||!is_int($hit['expiresAt'])||!array_key_exists('value',$hit))BehaviorBoundary::invalid('Invalid cache expiry/value.');
                    $now=(int)floor(microtime(true)*1000);
                    if($hit['expiresAt']>$now+$policy['ttlMs'])BehaviorBoundary::invalid('Cache expiry exceeds its policy.');
                    if($hit['expiresAt']>$now){$value=BehaviorBoundary::copy($hit['value'],$policy['maxBytes']);$execution->check();return $value;}
                }
            }
            $execution->check();$result=$this->resilient($record,$method,$metadata,$args,$original,$execution);
            $result=BehaviorBoundary::copy($result);
            if($key!==null) {
                $result=BehaviorBoundary::copy($result,$policy['maxBytes']);$execution->policy=$policy;$execution->expiresAt=(int)floor(microtime(true)*1000)+$policy['ttlMs'];
                BehaviorBoundary::adapter($this->adapter('caches',$policy['name'],'set'),[$key,BehaviorBoundary::copy($result),$execution],$execution);
            }
            $execution->check();return $result;
        } finally {$this->active--;}
    }
    private function resilient(array $record,string $method,array $metadata,array $args,callable $original,BehaviorExecution $execution): mixed {
        $limit=$metadata['concurrencyLimit']??null;$key=$record['name'].'.'.$method;
        if($limit!==null) {
            if(($this->slots[$key]??0)>=$limit['limit'])throw new BehaviorError('capacity','Behavior method concurrency limit reached.');
            $this->slots[$key]=($this->slots[$key]??0)+1;
        }
        try {
            $policy=$metadata['retryable']??null;
            for($attempt=0;;$attempt++) {
                try {
                    return $this->transacted($metadata,$args,$original,$execution);
                } catch(\Throwable $error) {
                    if($policy===null||$attempt>=$policy['maxRetries']||!BehaviorBoundary::retryable($error))throw $error;
                    $wait=BehaviorBoundary::backoff($policy,$attempt);
                    $execution->check();
                    if($wait>0)usleep($wait*1000);
                    $execution->check();
                }
            }
        } finally {
            if($limit!==null) {
                $this->slots[$key]--;
                if($this->slots[$key]<=0)unset($this->slots[$key]);
            }
        }
    }
    private function transacted(array $metadata,array $args,callable $original,BehaviorExecution $execution): mixed {
        $policy=$metadata['transactional']??null;
        if($policy===null||BehaviorOpenTransactions::of($policy['adapter'])!==null)return $this->body($metadata,$args,$original,$execution);
        $adapter=$this->adapter('entities',$policy['adapter'],'execute');
        $control=function(string $action,?string $identifier=null)use($adapter,$execution) {
            $command=['operation'=>'transaction','action'=>$action];
            if($identifier!==null)$command['transaction']=$identifier;
            return BehaviorBoundary::adapter($adapter,[$command,$execution],$execution);
        };
        $begun=BehaviorBoundary::copy($control('begin'),4096);
        $identifier=is_array($begun)?($begun['transaction']??null):null;
        if(!is_string($identifier)||$identifier===''||strlen($identifier)>128)BehaviorBoundary::invalid('A transactional adapter must answer a bounded transaction identifier.');
        try {
            $result=BehaviorOpenTransactions::within($policy['adapter'],$identifier,fn()=>$this->body($metadata,$args,$original,$execution));
        } catch(\Throwable $error) {
            try {$control('rollback',$identifier);} catch(\Throwable) {}
            throw $error;
        }
        $control('commit',$identifier);
        return $result;
    }
    private function body(array $metadata,array $args,callable $original,BehaviorExecution $execution): mixed {
        if(isset($metadata['mapping']))return BehaviorBoundary::mapValue($metadata['mapping'],$args);
        $policy=$metadata['httpExchange']??null;
        if($policy!==null&&($policy['generated']??false))return $this->exchange(array_merge($policy,['path'=>BehaviorBoundary::exchangePath($policy,$args)]),BehaviorBoundary::exchangeRequest($policy,$args),$execution);
        $result=$original($args);$execution->check();
        if($policy!==null)$result=$this->exchange($policy,$result,$execution);
        return $result;
    }
    private function exchange(array $policy,mixed $descriptor,BehaviorExecution $execution): mixed {
        BehaviorBoundary::keys($descriptor,['query','headers','body']);$descriptor=BehaviorBoundary::copy($descriptor);
        $query=$descriptor['query']??[];$headers=$descriptor['headers']??[];
        if(!is_array($query)||!is_array($headers))BehaviorBoundary::invalid('Invalid HTTP descriptor.');
        foreach($query as $value)if(!is_string($value)&&!is_int($value)&&!is_float($value)&&!is_bool($value))BehaviorBoundary::invalid('Invalid HTTP query.');
        foreach($headers as $name=>$value) {
            if(!is_string($name)||!preg_match('/^[!#$%&\x27*+.^_`|~0-9A-Za-z-]{1,128}$/D',$name)||!is_string($value)||strpbrk($value,"\r\n\0")!==false)BehaviorBoundary::invalid('Invalid HTTP header.');
            if(in_array(strtolower($name),['host','connection','content-length','transfer-encoding','cookie'],true))BehaviorBoundary::invalid('HTTP header is transport-owned.');
        }
        if(in_array($policy['method'],['GET','HEAD'],true)&&array_key_exists('body',$descriptor))BehaviorBoundary::invalid('HTTP reads cannot contain a request body.');
        $request=array_merge($descriptor,['method'=>$policy['method'],'path'=>$policy['path'],'query'=>$query,'headers'=>$headers,'redirect'=>'error','credentials'=>'omit','maxResponseBytes'=>1048576,'signal'=>$execution->signal,'deadline'=>$execution->deadline]);
        return BehaviorBoundary::copy(BehaviorBoundary::adapter($this->adapter('http',$policy['client'],'exchange'),[$request,$execution],$execution));
    }
    private function mutate(array $record,array $metadata,array $args,callable $original,BehaviorExecution $execution): mixed {
        $put=isset($metadata['cachePut']);$mutation=$metadata[$put?'cachePut':'cacheEvict'];$mapped=[];
        foreach($mutation['arguments'] as $index) {
            if(!array_key_exists($index,$args))BehaviorBoundary::invalid('Cache mutation argument is missing.');
            $mapped[]=$args[$index];
        }
        $target=$record['methods'][$mutation['method']];
        $mapped=$this->arguments($mapped,isset($target['pageable'])?['pageable'=>$target['pageable']]:[]);$policy=$record['cache'];
        $adapter=$this->adapter('caches',$policy['name'],$put?'set':'delete');
        $key=$this->key($record,$mutation['method'],$mapped,$execution,$put?'cache:put':'cache:evict');
        $execution->check();$result=$this->resilient($record,$mutation['method'],$metadata,$args,$original,$execution);
        $result=BehaviorBoundary::copy($result,$put?$policy['maxBytes']:1048576);$execution->check();
        if($put) {
            $execution->policy=$policy;$execution->expiresAt=(int)floor(microtime(true)*1000)+$policy['ttlMs'];
            BehaviorBoundary::adapter($adapter,[$key,BehaviorBoundary::copy($result),$execution],$execution);
        } else BehaviorBoundary::adapter($adapter,[$key,$execution],$execution);
        $execution->check();return $result;
    }
    private function query(array $policy,array $args,BehaviorExecution $execution): mixed {
        $where=[];
        foreach($policy['where'] as $predicate) {
            $entry=['field'=>$predicate['field'],'operator'=>$predicate['operator']];
            if(array_key_exists('argument',$predicate)) {
                if(!array_key_exists($predicate['argument'],$args))BehaviorBoundary::invalid('Query argument is missing.');
                $value=$args[$predicate['argument']];
                $scalar=static fn($item)=>$item===null||is_string($item)||is_bool($item)||is_int($item)||is_float($item);
                $valid=$predicate['operator']==='in' ? is_array($value)&&array_is_list($value)&&count($value)<=100&&count(array_filter($value,$scalar))===count($value) : $scalar($value);
                if(!$valid)BehaviorBoundary::invalid('Query requires finite scalar bindings or a bounded In array.');
                $entry['value']=$value;
            }
            $where[]=$entry;
        }
        $single=$policy['operation']==='findOne';
        $query=['kind'=>$single?'find':$policy['operation'],'where'=>BehaviorBoundary::copy($where,65536),'orderBy'=>$policy['orderBy']];
        if($single)$query['page']=BehaviorBoundary::page(['size'=>1],['defaultSize'=>1,'maxSize'=>1,'sort'=>$policy['fields']]);
        if($policy['operation']==='find') {
            $index=$policy['pageArgument'];
            if($index!==null&&!array_key_exists($index,$args))BehaviorBoundary::invalid('Query page argument is missing.');
            $page=BehaviorBoundary::page($index===null?[]:$args[$index],['defaultSize'=>$policy['defaultSize'],'maxSize'=>$policy['maxSize'],'sort'=>$policy['fields']]);
            if($page['sort'])BehaviorBoundary::invalid('Query ordering is annotation-owned.');
            $query['page']=$page;
        }
        $entity=$policy['entity'];$command=['operation'=>'query','entity'=>$entity['name'],'table'=>$entity['table'],'idField'=>$entity['id'],'query'=>$query];
        $joined=BehaviorOpenTransactions::of($entity['adapter']);
        if($joined!==null)$command['transaction']=$joined;
        $authorize=$this->adapters['authorizeEntity']??null;
        if(!is_callable($authorize))BehaviorBoundary::invalid('Store query requires entity authorization.');
        $adapter=$this->adapter('entities',$entity['adapter'],'execute');
        if(BehaviorBoundary::adapter($authorize,[BehaviorBoundary::copy($command),$execution],$execution)!==true)throw new BehaviorError('unauthorized','Store query was not authorized.');
        $result=BehaviorBoundary::copy(BehaviorBoundary::adapter($adapter,[BehaviorBoundary::copy($command),$execution],$execution));
        if($query['kind']==='find'&&(!is_array($result)||!array_is_list($result)||count($result)>$query['page']['size']||array_filter($result,static fn($row)=>!($row instanceof stdClass)&&(!is_array($row)||array_is_list($row)))))BehaviorBoundary::invalid('Query find result requires a bounded array of objects.');
        if(in_array($policy['operation'],['count','delete'],true)&&(!is_int($result)||$result<0||$result>9007199254740991))BehaviorBoundary::invalid('Query count and delete results require a nonnegative safe integer.');
        if($policy['operation']==='exists'&&!is_bool($result))BehaviorBoundary::invalid('Query exists result requires a boolean.');
        $execution->check();
        if($single)return $result[0]??null;
        return $result;
    }
    public function evict(object $instance,string $method,array $args): void {
        $record=$this->record($instance,get_class($instance),$method);
        if(!isset($record['methods'][$method]['cacheable']))BehaviorBoundary::invalid('Eviction requires a cacheable method.');
        $execution=$this->begin();
        try {$args=$this->arguments($args,$record['methods'][$method]);$key=$this->key($record,$method,$args,$execution,'cache:evict');BehaviorBoundary::adapter($this->adapter('caches',$record['cache']['name'],'delete'),[$key,$execution],$execution);}
        finally {$this->active--;}
    }
    public function restClient(string $name): BehaviorRestClient {
        if(!preg_match('/^[A-Za-z_$][A-Za-z0-9_$]{0,127}$/D',$name))BehaviorBoundary::invalid('RestClient requires a configured http adapter name.');
        return new BehaviorRestClient($this,$name);
    }
    public function restExchange(string $name,string $method,string $path,array $descriptor): mixed {
        $execution=$this->begin();
        try {return BehaviorBoundary::copy($this->exchange(['client'=>$name,'method'=>$method,'path'=>$path],$descriptor,$execution));}
        finally {$this->active--;}
    }
    public function repository(string $entity): BehaviorRepository {
        if(!isset($this->records[$entity]['entity']))BehaviorBoundary::invalid('Store requires source-local Entity/Table.');
        return new BehaviorRepository($this,$this->records[$entity]);
    }
    public function entityOperation(array $record,string $operation,mixed $value): mixed {
        if(($this->records[$record['name']]??null)!==$record)BehaviorBoundary::invalid('Entity belongs to another module.');
        if(!isset($record['entity'],$record['table'])||!in_array($operation,['find','save','delete'],true))BehaviorBoundary::invalid('Unknown entity operation.');
        if($operation==='find') {
            if(!is_array($value))BehaviorBoundary::invalid('Entity pagination requires a map.');
            $value=BehaviorBoundary::page($value,['defaultSize'=>20,'maxSize'=>100,'sort'=>[$record['entity']['id']]]);
        } elseif($operation==='save') {
            $value=BehaviorBoundary::copy($value);
            if(!is_array($value)&&!($value instanceof \stdClass))BehaviorBoundary::invalid('Entity value requires an object.');
            $fields=(array)$value;BehaviorBoundary::entityId($fields[$record['entity']['id']]??null);
        } else {BehaviorBoundary::entityId($value);}
        $execution=$this->begin();
        try {
            $command=['operation'=>$operation,'entity'=>$record['name'],'table'=>$record['table'],'idField'=>$record['entity']['id']];
            $command[$operation==='find'?'page':($operation==='save'?'value':'id')]=BehaviorBoundary::copy($value);
            $joined=BehaviorOpenTransactions::of($record['entity']['adapter']);
            if($joined!==null)$command['transaction']=$joined;
            return BehaviorBoundary::copy(BehaviorBoundary::adapter($this->adapter('entities',$record['entity']['adapter'],'execute'),[$command,$execution],$execution));
        } finally {$this->active--;}
    }
}
final class BehaviorRestClient {
    public function __construct(private readonly BehaviorContext $context,private readonly string $name) {}
    public function get(string $path): BehaviorRestCall { return new BehaviorRestCall($this->context,$this->name,'GET',$path); }
    public function post(string $path): BehaviorRestCall { return new BehaviorRestCall($this->context,$this->name,'POST',$path); }
    public function put(string $path): BehaviorRestCall { return new BehaviorRestCall($this->context,$this->name,'PUT',$path); }
    public function patch(string $path): BehaviorRestCall { return new BehaviorRestCall($this->context,$this->name,'PATCH',$path); }
    public function delete(string $path): BehaviorRestCall { return new BehaviorRestCall($this->context,$this->name,'DELETE',$path); }
    public function head(string $path): BehaviorRestCall { return new BehaviorRestCall($this->context,$this->name,'HEAD',$path); }
    public function options(string $path): BehaviorRestCall { return new BehaviorRestCall($this->context,$this->name,'OPTIONS',$path); }
}
final class BehaviorRestCall {
    private array $descriptor=['query'=>[],'headers'=>[]];
    public function __construct(private readonly BehaviorContext $context,private readonly string $name,
        private readonly string $method,private readonly string $path) {
        if(!BehaviorBoundary::safePath($path))BehaviorBoundary::invalid('RestClient requires a bounded origin-relative path without query, fragment or backslash.');
    }
    public function query(array $values): self { $this->descriptor['query']=array_merge($this->descriptor['query'],BehaviorBoundary::copy($values,4096)); return $this; }
    public function header(string $key,string $value): self { $this->descriptor['headers'][$key]=$value; return $this; }
    public function body(mixed $value): self {
        if(in_array($this->method,['GET','HEAD'],true))BehaviorBoundary::invalid('HTTP reads cannot contain a request body.');
        $this->descriptor['body']=$value; return $this;
    }
    public function retrieve(): mixed { return $this->context->restExchange($this->name,$this->method,$this->path,$this->descriptor); }
}
final class BehaviorRepository {
    public function __construct(private readonly BehaviorContext $context,private readonly array $record) {}
    public function find(array $page=[]): mixed {return $this->context->entityOperation($this->record,'find',BehaviorBoundary::page($page,['defaultSize'=>20,'maxSize'=>100,'sort'=>[$this->record['entity']['id']]]));}
    public function save(array|\stdClass $value): mixed {return $this->context->entityOperation($this->record,'save',$value);}
    public function delete(string|int $id): mixed {
        if((is_string($id)&&($id===''||strlen($id)>256))||(is_int($id)&&abs($id)>9007199254740991))BehaviorBoundary::invalid('Invalid entity identifier.');
        return $this->context->entityOperation($this->record,'delete',$id);
    }
}
final class BehaviorValidation {
    private static function error(string $path): never {throw new BehaviorError('validation','Behavior validation failed.',$path);}
    public static function apply(?array $schema,mixed $input): mixed {
        try {$value=BehaviorBoundary::copy($input);}catch(\Throwable $error){self::error('$.json');}
        if(!($value instanceof stdClass)&&(!is_array($value)||array_is_list($value)))self::error('$.object');
        if($schema===null||!isset($schema['fields']))self::error('$.schema');
        $fields=(array)$value;
        foreach($schema['fields'] as $field=>$rules) {
            $data=$fields[$field]??null;
            if(($rules['notNull']??false)&&$data===null)self::error($field.'.notNull');
            if(($rules['notBlank']??false)&&(!is_string($data)||preg_match('/^[\x{0009}-\x{000D}\x{0020}\x{0085}\x{00A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}]*$/uD',$data)))self::error($field.'.notBlank');
            if($data===null)continue;
            if(isset($rules['size'])) {
                $size=is_string($data)?strlen($data):(is_array($data)||$data instanceof stdClass?count((array)$data):-1);
                if($size<$rules['size']['min']||$size>$rules['size']['max'])self::error($field.'.size');
            }
            if(isset($rules['min'])&&((!is_int($data)&&!is_float($data))||$data<$rules['min']))self::error($field.'.min');
            if(isset($rules['max'])&&((!is_int($data)&&!is_float($data))||$data>$rules['max']))self::error($field.'.max');
        }
        return $value;
    }
}
final class BehaviorProperties {
    private static function error(string $code): never {throw new BehaviorError($code,'Invalid or inactive behavior configuration.');}
    public static function source(mixed $input,string $code): array {
        try {
            $values=BehaviorBoundary::copy($input,65536);
            if(is_array($values)&&(array_key_exists('tac',$values)||array_key_exists('yon',$values))) {
                $normalized=[];
                foreach(self::application($values) as $path=>$value)if(str_starts_with($path,'__BEHAVIOR_ENVIRONMENT__'.'.'))$normalized[strtoupper(str_replace('.','_',$path))]=$value;
                $values=$normalized;
            }
            if(!is_array($values)||count($values)>256)self::error($code);
            $prefix='__BEHAVIOR_ENVIRONMENT__'==='tac'?'TAC_':'YON_';
            foreach($values as $key=>$value) {
                if(!is_string($key)||strlen($key)>256||!str_starts_with($key,$prefix)||!preg_match('/^[A-Z][A-Z0-9_]*$/D',$key)||!is_string($value)||strlen($value)>4096)self::error($code);
            }
            return $values;
        } catch(\Throwable $error) {self::error($code);}
    }
    public static function select(array $record,array $options): array {
        $profiles=$options['profiles']??[];
        if(!is_array($profiles)||!array_is_list($profiles)||count($profiles)>32)self::error('configuration');
        foreach($profiles as $name)if(!is_string($name)||!preg_match('/^[A-Za-z_$][A-Za-z0-9_$]{0,63}$/D',$name))self::error('configuration');
        if(count(array_unique($profiles))!==count($profiles))self::error('configuration');
        $values=self::source($options['properties']??[],'configuration');
        if(isset($record['profiles'])&&!array_intersect($profiles,$record['profiles']))self::error('inactive-configuration');
        if(isset($record['conditionalOnProperty'])) {
            $condition=$record['conditionalOnProperty'];
            foreach($values as $key=>$_)if(str_starts_with($key,$condition['name'].'_'))self::error('inactive-configuration');
            $matches=array_key_exists($condition['name'],$values)?$values[$condition['name']]===$condition['havingValue']:$condition['matchIfMissing'];
            if(!$matches)self::error('inactive-configuration');
        }
        $options['profiles']=$profiles;$options['properties']=$values;return $options;
    }
    public static function bind(?array $policy,mixed $input): array {
        if($policy===null)self::error('configuration-properties');
        $values=self::source($input,'configuration-properties');$names=[];
        foreach($policy['fields'] as $descriptor)$names[]=$policy['prefix'].$descriptor['name'];
        foreach($values as $name=>$value)if(str_starts_with($name,$policy['prefix'])&&!in_array($name,$names,true))self::error('configuration-properties');
        $result=[];
        foreach($policy['fields'] as $field=>$descriptor) {
            $name=$policy['prefix'].$descriptor['name'];
            $result[$field]=self::value($descriptor,$values,$name);
        }
        return BehaviorBoundary::copy($result,65536);
    }
    public static function application(mixed $input): array {
        try {
            $application=BehaviorBoundary::copy($input,65536);
            $sections=is_array($application)?array_intersect_key($application,['tac'=>true,'yon'=>true]):[];
            if(!$sections)self::error('configuration-properties');
            $flat=[];
            foreach($sections as $root=>$value) {
                if(!is_array($value))self::error('configuration-properties');
                self::flatten($value,$root,0,$flat);
            }
            return $flat;
        } catch(\Throwable $error) {self::error('configuration-properties');}
    }
    private static function flatten(mixed $value,string $path,int $depth,array &$flat): void {
        if($depth>16||strlen($path)>256||count($flat)>=256)self::error('configuration-properties');
        if($path==='yon.server.port'&&!is_int($value))self::error('configuration-properties');
        if(is_array($value)) {
            if($depth>0&&$value===[]){$flat[$path.'.']='';return;}
            if($value!==[]&&array_is_list($value)){$flat[$path.'.']='';return;}
            foreach($value as $key=>$child) {
                if(!is_string($key)||!preg_match('/^[a-z][a-z0-9]*$/D',$key))self::error('configuration-properties');
                self::flatten($child,$path.'.'.$key,$depth+1,$flat);
            }
        } else {
            if($value===null){$flat[$path.'.']='';return;}
            if(!is_string($value)&&!is_bool($value)&&!is_int($value)&&!is_float($value))self::error('configuration-properties');
            $text=is_bool($value)?($value?'true':'false'):(string)$value;
            if(strlen($text)>4096||count($flat)>=256)self::error('configuration-properties');
            $flat[$path]=$text;
        }
    }
    public static function values(?array $policy,mixed $input): array {
        if($policy===null)self::error('configuration-properties');
        $flat=self::application($input);$result=[];
        foreach($policy as $field=>$descriptor)$result[$field]=self::value($descriptor,$flat,$descriptor['path']);
        return BehaviorBoundary::copy($result,65536);
    }
    private static function value(array $descriptor,array $values,string $name): mixed {
        $type=$descriptor['type'];
        foreach($values as $path=>$_)if(str_starts_with($path,$name.'.'))self::error('configuration-properties');
        if(!array_key_exists($name,$values)) {
            if(!array_key_exists('default',$descriptor))self::error('configuration-properties');
            $value=$descriptor['default'];
        } elseif($type==='string')$value=$values[$name];
        elseif($type==='boolean') {
            if(!in_array($values[$name],['true','false'],true))self::error('configuration-properties');
            $value=$values[$name]==='true';
        } else {
            $grammar=$type==='integer'?'/^-?(0|[1-9][0-9]*)$/D':'/^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?$/D';
            if(!preg_match($grammar,$values[$name]))self::error('configuration-properties');
            $value=(float)$values[$name];
        }
        if($type==='string'&&(!is_string($value)||strlen($value)>4096))self::error('configuration-properties');
        if(in_array($type,['integer','number'],true)) {
            if((!is_int($value)&&!is_float($value))||!is_finite((float)$value)||abs($value)>9007199254740991||($type==='integer'&&floor($value)!=$value)||(isset($descriptor['min'])&&$value<$descriptor['min'])||(isset($descriptor['max'])&&$value>$descriptor['max']))self::error('configuration-properties');
            if($type==='integer')$value=(int)$value;
        }
        return $value;
    }
}
final class Behaviors {
    private static function records(): array {return json_decode('__BEHAVIOR_RECORDS__',true,32,JSON_THROW_ON_ERROR);}
    public static function create(string $configuration,array $options): BehaviorContext {
        $records=self::records();if(!($records[$configuration]['configuration']??false))BehaviorBoundary::invalid('Setup must be source-local.');
        $options=BehaviorProperties::select($records[$configuration],$options);
        $context=new BehaviorContext($records,$options);
        try {
            $setupKey=self::setupKey($configuration,$options);
            if(!isset(self::$setups[$setupKey]))self::$setups[$setupKey]=['adapters'=>self::configuredAdapters($context,$configuration),'users'=>0];
            self::$setups[$setupKey]['users']++;
            $context->share($setupKey,self::$setups[$setupKey]['adapters']);
            return $context;
        }
        catch(\Throwable $error){$context->dispose();throw $error;}
    }
    private static array $setups=[];
    private static function setupKey(string $configuration,array $options): string {
        $profiles=$options['profiles']??[];sort($profiles);$properties=$options['properties']??[];ksort($properties);
        return json_encode([$configuration,$profiles,$properties],JSON_THROW_ON_ERROR);
    }
    private static function configuredAdapters(BehaviorContext $context,string $configuration): array {
        $context->configure(function($execution)use($configuration){switch($configuration){__BEHAVIOR_CONFIGURATIONS__}BehaviorBoundary::invalid('Unknown configuration.');});
        return $context->adapters();
    }
    public static function releaseSetup(string $setupKey): void {
        if(!isset(self::$setups[$setupKey]))return;
        self::$setups[$setupKey]['users']--;
        if(self::$setups[$setupKey]['users']===0)unset(self::$setups[$setupKey]);
    }
    public static function construct(string $type,array $values): object {
        $fields=self::records()[$type]['constructor']??null;
        if($fields===null||count($fields)!==count($values)||array_diff(array_keys($values),$fields))BehaviorBoundary::invalid('Constructor fields must exactly match their declaration.');
        switch($type){__BEHAVIOR_CONSTRUCTORS__}
        BehaviorBoundary::invalid('Unknown constructor.');
    }
    public static function properties(string $type,array $values): array {return BehaviorProperties::bind(self::records()[$type]['configurationProperties']??null,$values);}
    public static function values(string $type,array $application): array {return BehaviorProperties::values(self::records()[$type]['values']??null,$application);}
    public static function validate(string $type,mixed $value): mixed {return BehaviorValidation::apply(self::records()[$type]['validated']??null,$value);}
}
final class BehaviorCalls {
    private static array $keys = [];
    public static function record(string $key): void { self::$keys[$key] = true; }
    public static function take(): array { $keys = array_map('strval', array_keys(self::$keys)); self::$keys = []; return $keys; }
}
