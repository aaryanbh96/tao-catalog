#!/usr/bin/env python3
"""
One-time import of the cleaned catalog.xlsx into Postgres.

Safety guarantees:
  * Every SKU is read and inserted as TEXT — leading zeros like '09100' are preserved.
  * Each row is validated before insert; problems are collected and reported, and the
    whole import runs in a single transaction (all-or-nothing) so a bad file can't leave
    a half-loaded database.
  * Category and stock/sync/website values are checked against the allowed enum values.

Usage:
    python import_catalog.py path/to/catalog.xlsx "postgresql://user:pass@host:5432/db"
"""
import sys
import pandas as pd
import psycopg2
from psycopg2.extras import execute_values

VALID_CATEGORY = {'Tea', 'Teaware', 'Accessories', 'Gift Sets', 'Other'}
VALID_STOCK    = {'In Stock', 'Out of Stock', 'Discontinued', 'Unknown'}
VALID_SYNC     = {'Synced', 'Needs update', 'Done'}
VALID_WEB      = {'Live', 'Hidden', 'OOS'}


def parse_price(raw):
    """'$13.00' -> 13.00 ; '' -> None. Never raises."""
    s = str(raw or '').strip().replace('$', '').replace(',', '')
    if not s:
        return None
    # take the first number if it's a range like '3.00-12.00'
    s = s.split('-')[0].split('–')[0].strip()
    try:
        return round(float(s), 2)
    except ValueError:
        return None


def parse_qty(raw):
    s = str(raw or '').strip()
    if not s:
        return None
    try:
        return int(float(s))
    except ValueError:
        return None


def load_rows(xlsx_path):
    # keep_default_na=False + dtype=str => nothing gets coerced; '09100' stays '09100'
    df = pd.read_excel(xlsx_path, dtype=str, keep_default_na=False).fillna('')
    df.columns = [c.strip() for c in df.columns]
    return df


def validate(df):
    """Return (errors, warnings). Errors block import; warnings are reported only."""
    errors, warnings = [], []
    for i, r in df.iterrows():
        row = i + 2  # Excel row number
        psku = r['Parent SKU'].strip()
        vsku = r['Variation SKU'].strip()
        if not psku:
            errors.append(f"Row {row}: blank Parent SKU ({r['Product Name']!r})")
        if not vsku:
            errors.append(f"Row {row}: blank Variation SKU ({r['Product Name']!r})")
        if not r['Product Name'].strip():
            errors.append(f"Row {row}: blank Product Name")
        cat = r['Category'].strip()
        if cat and cat not in VALID_CATEGORY:
            errors.append(f"Row {row}: category {cat!r} not allowed; pick from {sorted(VALID_CATEGORY)}")
        st = r['Stock Status'].strip()
        if st and st not in VALID_STOCK:
            errors.append(f"Row {row}: stock status {st!r} not allowed")
        # mangled-SKU heuristic: a SKU that's all digits and shorter than its siblings often
        # means a lost leading zero — flag, don't block.
        if vsku and vsku.isdigit() and len(vsku) == 4:
            warnings.append(f"Row {row}: variation SKU {vsku!r} is 4 digits — check it didn't lose a leading zero")
        if parse_price(r['Price']) is None and r['Price'].strip():
            warnings.append(f"Row {row}: price {r['Price']!r} could not be parsed -> stored as empty")
    # duplicate variation SKUs
    dups = df[df['Variation SKU'].str.strip() != ''].groupby(df['Variation SKU'].str.strip()).size()
    for sku, n in dups[dups > 1].items():
        errors.append(f"Variation SKU {sku!r} appears {n} times — must be unique")
    return errors, warnings


def build_records(df):
    """Collapse rows into products + variations, keyed by Parent SKU."""
    products = {}   # parent_sku -> product dict
    variations = []
    for _, r in df.iterrows():
        psku = r['Parent SKU'].strip()
        if psku not in products:
            products[psku] = {
                'parent_sku': psku,
                'name': r['Product Name'].strip(),
                'category': r['Category'].strip() or 'Tea',
                'platform': r['Platform'].strip() or 'Tao of Tea (own site)',
                'url': r['URL'].strip() or None,
                'image_url': r['Image URL'].strip() or None,
                'short_description': r['Short Description'].strip() or None,
                'notes': r['Notes'].strip() or None,
            }
        variations.append({
            'parent_sku': psku,
            'variation_sku': r['Variation SKU'].strip(),
            'variation_name': r['Variation'].strip() or 'Standard',
            'price': parse_price(r['Price']),
            'qty': parse_qty(r['Qty']),
            'stock': r['Stock Status'].strip() or 'In Stock',
            'web_status': r['Website Status'].strip() or 'Live',
            'sync': r['Sync State'].strip() or 'Synced',
        })
    return list(products.values()), variations


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    xlsx_path = sys.argv[1]
    if len(sys.argv) >= 3:
        dsn = sys.argv[2]
    else:
        with open('db_url.txt', 'r', encoding='utf-8') as f:
            dsn = f.read().strip()

    df = load_rows(xlsx_path)
    print(f"Read {len(df)} rows.")

    errors, warnings = validate(df)
    if warnings:
        print(f"\n{len(warnings)} warning(s):")
        for w in warnings:
            print("  ⚠", w)
    if errors:
        print(f"\n{len(errors)} ERROR(s) — import aborted, nothing written:")
        for e in errors:
            print("  ✗", e)
        sys.exit(1)

    products, variations = build_records(df)
    print(f"\n{len(products)} products, {len(variations)} variations to import.")

    conn = psycopg2.connect(dsn)
    try:
        with conn:  # single transaction: commits on success, rolls back on any error
            with conn.cursor() as cur:
                execute_values(cur, """
                    INSERT INTO products (parent_sku, name, category, platform, url, image_url, short_description, notes)
                    VALUES %s RETURNING id, parent_sku
                """, [(p['parent_sku'], p['name'], p['category'], p['platform'],
                       p['url'], p['image_url'], p['short_description'], p['notes']) for p in products],
                    page_size=len(products) + 1)  # one page, so RETURNING captures every row
                id_by_sku = {sku: pid for pid, sku in cur.fetchall()}

                execute_values(cur, """
                    INSERT INTO variations (product_id, variation_sku, variation_name, price, qty, stock, web_status, sync)
                    VALUES %s
                """, [(id_by_sku[v['parent_sku']], v['variation_sku'], v['variation_name'],
                       v['price'], v['qty'], v['stock'], v['web_status'], v['sync']) for v in variations])
        print("✓ Import committed successfully.")
    finally:
        conn.close()


if __name__ == '__main__':
    main()
