@Service
@Lifetime({"value":"prototype"})
export default class HarnessService {
  first = true
  second = false
  script = false
}
