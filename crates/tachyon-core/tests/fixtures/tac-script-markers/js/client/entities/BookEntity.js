@Entity({"adapter":"shelf"})
@Table('books')
export default class BookEntity {
  /** @type {string} */
  @Id
  code;
  /** @type {string} */
  status;
}
