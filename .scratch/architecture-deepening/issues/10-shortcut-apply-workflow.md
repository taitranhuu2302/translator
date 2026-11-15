# 10: Shortcut apply — one sequence for rebind and reset

**What to build:** The apply half of shortcut rebinding moves behind ticket 08's seam as one sequence: register the new binding, unregister the old one, set the route, save the settings, and on any failure after the old binding is gone, undo both — unregister the new binding, re-register the old one, restore the old route. Resetting shortcuts reuses the same sequence instead of repeating it. The pure planning half is unchanged.

**Blocked by:** 08 (Shell workflow seam — and start at login as its first workflow)

**Status:** ready-for-agent

- [ ] The apply sequence is one function over the registry and route ports, in the stated order, with the rollback stated once.
- [ ] Reset reuses that sequence; there is no second copy of the ordering.
- [ ] The pure planning function and its behaviour are untouched.
- [ ] Tests assert the order through fakes, and assert that a forced save failure leaves the old binding registered, the new one unregistered, and the route back where it was.
- [ ] A successful rebind leaves exactly one binding registered for the action.
- [ ] A shortcut that the system refuses to register is reported with the existing wording and changes nothing.
