final class TacNeeds {
    private static final String FAILED_START = "A Java service that another service needs could not start.";
    private static final java.util.Map<String, TacNode> SHARED = new java.util.LinkedHashMap<>();
    private static final java.util.List<TacNode> ORDER = new java.util.ArrayList<>();
    private static TacNode building;

    private TacNeeds() {
    }

    static Object need(String name) {
        TacNode needed = obtain(name);
        if (building != null) {
            building.needs.add(needed);
        }
        if (!start(needed)) {
            throw new IllegalStateException(FAILED_START);
        }
        return needed.service;
    }

    static TacNode attach(String name, TacActivation owner) {
        if (TacServiceTable.shared(name)) {
            return obtain(name);
        }
        TacNode made = build(name, owner.preferences, owner);
        made.holders = 1;
        return made;
    }

    static boolean start(TacNode node) {
        if (node.started) {
            return !node.failed;
        }
        node.started = true;
        try {
            node.runHooks(false);
        } catch (RuntimeException problem) {
            node.failed = true;
            throw problem;
        }
        return true;
    }

    static void release(TacNode held) {
        held.holders--;
        boolean parks = SHARED.get(held.name) == held && !held.failed;
        if (held.holders > 0 || parks) {
            return;
        }
        finish(held);
    }

    static void sweepWhen(boolean nobodyHolds) {
        if (nobodyHolds) {
            sweep();
        }
    }

    private static void sweep() {
        for (int index = ORDER.size() - 1; index >= 0; index--) {
            TacNode parked = ORDER.get(index);
            if (parked.holders == 0 && SHARED.get(parked.name) == parked) {
                finish(parked);
            }
        }
        ORDER.clear();
    }

    private static TacNode obtain(String name) {
        TacNode existing = SHARED.get(name);
        if (existing != null) {
            existing.holders++;
            return existing;
        }
        TacNode made = build(name, new TacPreferenceScope(), null);
        made.holders = 1;
        SHARED.put(name, made);
        ORDER.add(made);
        return made;
    }

    private static TacNode build(String name, TacPreferenceScope preferences, TacActivation owner) {
        TacNode made = new TacNode(name, preferences);
        TacNode outer = building;
        building = made;
        try {
            TacServiceTable.build(made, owner);
        } catch (RuntimeException problem) {
            releaseNeeds(made);
            throw problem;
        } finally {
            building = outer;
        }
        return made;
    }

    private static void finish(TacNode held) {
        if (SHARED.get(held.name) == held) {
            SHARED.remove(held.name);
        }
        held.preferences.dispose();
        try {
            if (held.started && held.members != null) {
                held.runHooks(true);
            }
        } finally {
            releaseNeeds(held);
        }
    }

    private static void releaseNeeds(TacNode held) {
        java.util.List<TacNode> needs = new java.util.ArrayList<>(held.needs);
        held.needs.clear();
        for (int index = needs.size() - 1; index >= 0; index--) {
            release(needs.get(index));
        }
    }
}
