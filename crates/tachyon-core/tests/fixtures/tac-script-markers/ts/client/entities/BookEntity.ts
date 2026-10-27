@Entity({"adapter":"shelf"})
@Table('books')
export default class BookEntity {
  @Id
  code!: string;
  status!: string;
}
