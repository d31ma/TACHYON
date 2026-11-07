interface TacCall {
    Object run() throws Exception;

    static Object result(TacCall call) {
        try {
            return call.run();
        } catch (RuntimeException problem) {
            throw problem;
        } catch (Exception problem) {
            throw new IllegalStateException(problem.getMessage(), problem);
        }
    }
}
