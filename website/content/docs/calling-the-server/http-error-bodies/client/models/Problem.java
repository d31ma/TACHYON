import models.ProblemEntry;
record Problem(String title, int status, java.util.List<ProblemEntry> errors) {}
