import models.ShortDetail;
record Summary(int id, String name, ShortDetail detail, java.util.List<ShortDetail> items, String kind) {}
