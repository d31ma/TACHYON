import models.Detail;
record Product(int id, String name, Detail detail, java.util.List<Detail> items, String secret) {}
