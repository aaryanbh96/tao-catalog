#!/usr/bin/env python3
"""
Smoke-test the running service. Start the service first (cargo run), then:
    python scripts/test_service.py
Hits every endpoint against http://localhost:8080 and prints pass/fail.
Read-only except for a stock toggle that it sets and then reverts.
"""
import json
import sys
import urllib.request

BASE = "http://localhost:8080"


def call(method, path, body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(BASE + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req) as r:
        txt = r.read().decode()
        return r.status, (json.loads(txt) if txt else None)


def main():
    ok = True

    # health
    try:
        st, _ = call("GET", "/health")
        print(f"[{'PASS' if st == 200 else 'FAIL'}] GET /health -> {st}")
    except Exception as e:
        print(f"[FAIL] GET /health -> {e}\n  Is the service running? (cargo run)")
        sys.exit(1)

    # products
    st, products = call("GET", "/products")
    print(f"[{'PASS' if st == 200 else 'FAIL'}] GET /products -> {st}, {len(products)} products")
    if not products:
        print("  No products — did the import run?")
        sys.exit(1)

    # pick a multi-variation product to exercise variations + stock toggle
    target = next((p for p in products if p["variation_count"] > 1), products[0])
    pid = target["id"]
    st, vars_ = call("GET", f"/products/{pid}/variations")
    print(f"[{'PASS' if st == 200 else 'FAIL'}] GET /products/{pid}/variations -> {st}, "
          f"{len(vars_)} variations of {target['name']!r}")

    # stock toggle: flip one to Out of Stock, confirm trigger set sync, then revert
    v = vars_[0]
    orig = v["stock"]
    st, updated = call("PATCH", f"/variations/{v['id']}/stock", {"stock": "Out of Stock"})
    trigger_ok = updated and updated["sync"] == "Needs update" and updated["web_status"] == "OOS"
    print(f"[{'PASS' if st == 200 and trigger_ok else 'FAIL'}] PATCH stock -> {st}; "
          f"trigger set sync={updated['sync']!r}, web_status={updated['web_status']!r}")

    # queue should now contain it
    st, queue = call("GET", "/queue")
    in_queue = any(q["variation_id"] == v["id"] for q in queue)
    print(f"[{'PASS' if st == 200 and in_queue else 'FAIL'}] GET /queue -> {st}, "
          f"{len(queue)} pending; our item present: {in_queue}")

    # mark synced
    st, _ = call("POST", f"/variations/{v['id']}/synced")
    print(f"[{'PASS' if st == 200 else 'FAIL'}] POST /variations/{v['id']}/synced -> {st}")

    # revert stock to original so the test leaves no trace
    call("PATCH", f"/variations/{v['id']}/stock", {"stock": orig})
    call("POST", f"/variations/{v['id']}/synced")
    print(f"[INFO] reverted {v['variation_sku']} back to {orig!r}")

    # invalid stock value should be rejected
    try:
        st, _ = call("PATCH", f"/variations/{v['id']}/stock", {"stock": "banana"})
        print(f"[FAIL] invalid stock accepted -> {st} (should be 400)")
    except urllib.error.HTTPError as e:
        print(f"[{'PASS' if e.code == 400 else 'FAIL'}] invalid stock rejected -> {e.code}")

    print("\nDone.")


if __name__ == "__main__":
    main()
