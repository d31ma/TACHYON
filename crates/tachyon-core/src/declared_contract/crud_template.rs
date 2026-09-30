#[derive(Clone, Copy)]
pub(crate) struct CrudTemplate {
    pub(crate) language: &'static str,
    pub(crate) id: &'static str,
    pub(crate) members: &'static str,
}

impl CrudTemplate {
    const JAVA_MEMBERS: &'static str = "Object findById({id} id) {}\nObject findAll(java.util.Map<String, Object> page) {}\nObject deleteById({id} id) {}\nObject save(Object entity) {\n    return behaviors.repository(\"{entity}\").save(entity);\n}";
    const TEMPLATES: [Self; 13] = [
        Self::new("java", "Object", Self::JAVA_MEMBERS),
        Self::new("groovy", "Object", Self::JAVA_MEMBERS),
        Self::new(
            "kt",
            "Any?",
            "fun findById(id: {id}): Any? {}\nfun findAll(page: Map<String, Any?>): Any? {}\nfun deleteById(id: {id}): Any? {}\nfun save(entity: Any?): Any? = behaviors.repository(\"{entity}\").save(entity)",
        ),
        Self::new(
            "cs",
            "BehaviorJSON",
            "public async System.Threading.Tasks.Task<BehaviorJSON> findById({id} id) {}\npublic async System.Threading.Tasks.Task<BehaviorJSON> findAll(BehaviorJSON page) {}\npublic async System.Threading.Tasks.Task<BehaviorJSON> deleteById({id} id) {}\npublic System.Threading.Tasks.Task<BehaviorJSON> save(BehaviorJSON entity) => behaviors.repository(typeof({entity})).save(entity);",
        ),
        Self::new(
            "php",
            "int|string",
            "public function findById({id} $id): ?array {}\npublic function findAll(array $page): array {}\npublic function deleteById({id} $id): int {}\npublic function save(array|\\stdClass $entity): mixed {\n    return $this->behaviors->repository({entity}::class)->save($entity);\n}",
        ),
        Self::new(
            "go",
            "BehaviorValue",
            "\nfunc ({receiver} *{owner}) FindById(id BehaviorValue) (BehaviorValue, error) {\n}\n\nfunc ({receiver} *{owner}) FindAll(page BehaviorValue) (BehaviorValue, error) {\n}\n\nfunc ({receiver} *{owner}) DeleteById(id BehaviorValue) (BehaviorValue, error) {\n}\n\nfunc ({receiver} *{owner}) Save(entity BehaviorValue) (BehaviorValue, error) {\n\trepository, err := BehaviorsRepository[{entity}]({receiver}.behaviors)\n\tif err != nil {\n\t\treturn nil, err\n\t}\n\treturn repository.Save(entity)\n}\n",
        ),
        Self::new(
            "dart",
            "Object?",
            "Future<Object?> findById({id} id) async {}\n\nFuture<Object?> findAll(Map<String, Object?> page) async {}\n\nFuture<Object?> deleteById({id} id) async {}\n\nFuture<Object?> save(Object? entity) => behaviors.repository({entity}).save(entity);",
        ),
        Self::new(
            "rs",
            "BehaviorValue",
            "fn find_by_id(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {}\nfn find_all(&self, page: BehaviorValue) -> BehaviorResult<BehaviorValue> {}\nfn delete_by_id(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {}\nfn save(&self, entity: BehaviorValue) -> BehaviorResult<BehaviorValue> {\n    self.behaviors.repository::<{entity}>()?.save(entity)\n}",
        ),
        Self::new(
            "swift",
            "BehaviorJSON",
            "func findById(_ id: BehaviorJSON) async throws -> BehaviorJSON {}\nfunc findAll(_ page: BehaviorJSON) async throws -> BehaviorJSON {}\nfunc deleteById(_ id: BehaviorJSON) async throws -> BehaviorJSON {}\nfunc save(_ entity: BehaviorJSON) async throws -> BehaviorJSON {\n    try await behaviors.repository({entity}.self).save(entity)\n}",
        ),
        Self::new(
            "py",
            "object",
            "async def find_by_id(self, id): ...\n\nasync def find_all(self, page): ...\n\nasync def delete_by_id(self, id): ...\n\nasync def save(self, entity):\n    return await self.behaviors.repository({entity}).save(entity)",
        ),
        Self::new(
            "js",
            "",
            "async findById(id) {}\nasync findAll(page) {}\nasync deleteById(id) {}\nasync save(entity) {\n  return this.behaviors.repository({entity}).save(entity)\n}",
        ),
        Self::new(
            "ts",
            "string | number",
            "async findById(id: {id}): Promise<{entity} | null | void> {}\nasync findAll(page: BehaviorPageInput): Promise<{entity}[] | void> {}\nasync deleteById(id: {id}): Promise<number | void> {}\nasync save(entity: {entity}) {\n  return this.behaviors.repository({entity}).save(entity)\n}",
        ),
        Self::new(
            "rb",
            "",
            "def find_by_id(id)\nend\n\ndef find_all(page)\nend\n\ndef delete_by_id(id)\nend\n\ndef save(entity)\n  @behaviors.repository({entity}).save(entity)\nend",
        ),
    ];

    const fn new(language: &'static str, id: &'static str, members: &'static str) -> Self {
        Self {
            language,
            id,
            members,
        }
    }

    pub(crate) fn find(language: &str) -> Option<Self> {
        Self::TEMPLATES
            .into_iter()
            .find(|template| template.language == language)
    }
}
