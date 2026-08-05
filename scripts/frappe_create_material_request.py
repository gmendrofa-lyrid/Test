"""Frappe API Server Script: validate and create a Material Request with an explicit owner.

`frappe` is supplied by the Server Script runtime; paste the body into the API script as-is.
Successful POST requests are committed by Frappe, so this script deliberately does not commit
manually and leaves rollback behavior intact when validation fails.
"""

raw_body = frappe.request.get_data()
data = json.loads(raw_body or b"{}")

if not isinstance(data, dict):
    frappe.throw("Request body must be a JSON object.", frappe.ValidationError)

current_user = frappe.session.user
system_manager_role = frappe.db.get_value(
    "Has Role",
    {
        "parent": current_user,
        "parenttype": "User",
        "role": "System Manager",
    },
    "name",
)
is_system_manager = bool(system_manager_role)
if current_user != "sysadmin@bahteraadijaya.com" and not is_system_manager:
    frappe.throw("Only System Manager can use this API.", frappe.PermissionError)

target_owner = (data.pop("owner", None) or "").strip()
if not target_owner:
    frappe.throw("Field owner is required.", frappe.ValidationError)
if target_owner == "Guest":
    frappe.throw("Guest cannot be used as document owner.", frappe.ValidationError)

target_user = frappe.db.get_value(
    "User",
    target_owner,
    ["name", "enabled"],
    as_dict=True,
)
if not target_user:
    frappe.throw("User %s does not exist." % target_owner, frappe.ValidationError)
if not target_user.enabled:
    frappe.throw("User %s is disabled." % target_owner, frappe.ValidationError)

material_request_type = data.get("material_request_type")
if not material_request_type:
    frappe.throw("Field material_request_type is required.", frappe.ValidationError)

incoming_items = data.get("items")
if not isinstance(incoming_items, list) or not incoming_items:
    frappe.throw("At least one Material Request item is required.", frappe.ValidationError)

requested_item_codes = []
for index, incoming_item in enumerate(incoming_items, start=1):
    if not isinstance(incoming_item, dict):
        frappe.throw(
            "Item row %s must be a JSON object." % index,
            frappe.ValidationError,
        )
    item_code = (incoming_item.get("item_code") or "").strip()
    if not item_code:
        frappe.throw(
            "Item row %s: item_code is required." % index,
            frappe.ValidationError,
        )
    requested_item_codes.append(item_code)

item_master_rows = frappe.get_all(
    "Item",
    filters={"name": ["in", requested_item_codes]},
    fields=["name", "country_of_origin"],
    limit_page_length=1000,
)
origin_by_item = {}
for item_master_row in item_master_rows:
    origin_by_item[item_master_row.get("name")] = item_master_row.get("country_of_origin")

# `origin` is mandatory on the BAJ Material Request Item customization. Item master data is the
# source of truth: reject the entire request if an Item is missing or has no country_of_origin.
missing_item_codes = []
missing_origin_item_codes = []
for item_code in requested_item_codes:
    if item_code not in origin_by_item:
        if item_code not in missing_item_codes:
            missing_item_codes.append(item_code)
    elif not str(origin_by_item.get(item_code) or "").strip():
        if item_code not in missing_origin_item_codes:
            missing_origin_item_codes.append(item_code)

if missing_item_codes:
    frappe.throw(
        "Cannot create Material Request. These Items do not exist: %s"
        % ", ".join(missing_item_codes),
        frappe.ValidationError,
    )

if missing_origin_item_codes:
    frappe.throw(
        "Cannot create Material Request. Update Country of Origin in the Item master for: %s"
        % ", ".join(missing_origin_item_codes),
        frappe.ValidationError,
    )

allowed_item_fields = [
    "item_code",
    "schedule_date",
    "qty",
    "stock_qty",
    "uom",
    "stock_uom",
    "shipment",
    "origin",
    "warehouse",
    "conversion_factor",
    "rate",
    "remark",
    "custom_remark_2",
    "from_warehouse",
]
clean_items = []
for index, incoming_item in enumerate(incoming_items, start=1):
    item_code = (incoming_item.get("item_code") or "").strip()
    try:
        quantity = float(incoming_item.get("qty") or 0)
    except (TypeError, ValueError):
        frappe.throw(
            "Item row %s: qty must be numeric." % index,
            frappe.ValidationError,
        )
    if quantity <= 0:
        frappe.throw(
            "Item row %s: qty must be greater than zero." % index,
            frappe.ValidationError,
        )

    origin = str(origin_by_item.get(item_code) or "").strip()

    clean_item = {}
    for fieldname in allowed_item_fields:
        if fieldname in incoming_item:
            clean_item[fieldname] = incoming_item[fieldname]
    clean_item["item_code"] = item_code
    clean_item["qty"] = quantity
    clean_item["origin"] = origin
    try:
        stock_quantity = float(clean_item.get("stock_qty") or 0)
    except (TypeError, ValueError):
        stock_quantity = 0
    if stock_quantity <= 0:
        clean_item["stock_qty"] = quantity
    clean_items.append(clean_item)

allowed_parent_fields = [
    "ndk_branch",
    "material_request_type",
    "item_group",
    "transaction_date",
    "company",
    "company_series",
    "set_from_warehouse",
    "set_warehouse",
    "reference_doctype",
    "reference_name",
]
document_data = {"doctype": "Material Request", "items": clean_items}
for fieldname in allowed_parent_fields:
    if fieldname in data:
        document_data[fieldname] = data[fieldname]
document_data["company_series"] = "TEST-"

doc = frappe.get_doc(document_data)
doc.owner = target_owner
doc.insert(ignore_permissions=True)

frappe.response["message"] = "RO created!"
frappe.response["name"] = doc.name
