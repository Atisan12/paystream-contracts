/**
 * @name Direct storage mutation without require_auth in Soroban contract
 * @description A contract function calls env.storage().*.set() or .remove()
 *              without a require_auth guard in the same function body.
 *              This is a more precise companion to MissingRequireAuth.ql that
 *              targets direct SDK storage writes rather than helper-function calls.
 * @kind problem
 * @problem.severity warning
 * @security-severity 8.0
 * @precision medium
 * @id rust/soroban/storage-mutation-without-auth
 * @tags security
 *       soroban
 */

import rust

/**
 * Holds if `f` contains a method call chain that terminates in `.set(..)`
 * or `.remove(..)` on a Soroban storage object
 * (e.g., `env.storage().instance().set(..)`).
 */
predicate writesStorage(Function f) {
  exists(MethodCallExpr mc |
    mc.getEnclosingCallable() = f and
    mc.getIdentifier().getText() = ["set", "remove"] and
    // The receiver should itself be a method call (chained call).
    mc.getReceiver() instanceof MethodCallExpr
  )
}

/**
 * Holds if `f` calls `require_auth` or `require_auth_for_args` anywhere in
 * its body (including through a method call expression on any receiver).
 */
predicate hasRequireAuth(Function f) {
  exists(MethodCallExpr mc |
    mc.getEnclosingCallable() = f and
    mc.getIdentifier().getText() = ["require_auth", "require_auth_for_args"]
  )
}

from Function f
where
  // Only consider functions that are direct members of an impl block
  // (i.e. potential #[contractimpl] entry points).
  exists(Impl i | f = i.getAssocItemList().getAnAssocItem()) and
  // Public visibility only.
  exists(f.getVisibility()) and
  writesStorage(f) and
  not hasRequireAuth(f)
select f,
  "Contract function '" + f.getName().getText() +
    "' writes to storage directly without calling require_auth."
