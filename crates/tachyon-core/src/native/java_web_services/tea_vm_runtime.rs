pub(super) struct TeaVmRuntime;

impl TeaVmRuntime {
    const CODE_FROM_TEXT: [(&'static str, &'static str); 5] = [
        (
            r#"function D(e){let t=n.get(e);if(typeof t==="undefined"){t=new Function("return "+e+";");n.set(e,t)}return t()}function O(e,t){new Function("value",e+" = value;")(t)}"#,
            r#"function D(e){let t=n.get(e);if(typeof t==="undefined"){t=()=>e.split(".").reduce((r,o)=>r[o],globalThis);n.set(e,t)}return t()}function O(e,t){const r=e.split(".");const o=r.pop();r.reduce((a,l)=>a[l],globalThis)[o]=t}"#,
        ),
        (
            r#"function S(t,e){const n=[];const r=[];for(let e=0;e<t.length;++e){const a="p"+e;n.push(a);r.push(a)}if(e){const s=r.length-1;r[s]="..."+r[s]}const o=r.join(", ");return new Function("rethrowJavaAsJs","fn",`return function(${o}) {
    try {
        return fn(${n.join(", ")});
    } catch (e) {
        rethrowJavaAsJs(e);
    }
};`)(j,t)}"#,
            r#"function S(t,e){const c=t.length;return function(...a){const p=a.slice(0,e?c-1:c);p.length=e?c-1:c;if(e){p.push(a.slice(c-1))}try{return t(...p)}catch(r){j(r)}}}"#,
        ),
        (
            r#"function W(e,t){return new Function("constructor",`return function ${e}(marker, javaObject) {
    return constructor.call(this, marker, javaObject);
}
`)(t)}"#,
            r#"function W(e,t){return{[e]:function(r,o){return t.call(this,r,o)}}[e]}"#,
        ),
        (
            r#"defineMethod(e,t,n,r){const o=[];const a=[];for(let e=1;e<n.length;++e){const l="p"+e;o.push(l);a.push(l)}if(r){const c=a.length-1;a[c]="..."+a[c]}const s=a.join(", ");e.prototype[t]=new Function("rethrowJavaAsJs","fn",`return function(${s}) {
    try {
        return fn(${["this",...o].join(", ")});
    } catch (e) {
        rethrowJavaAsJs(e);
    }
};`)(j,n)}"#,
            r#"defineMethod(e,t,n,r){const c=n.length-1;e.prototype[t]=function(...a){const p=a.slice(0,r?c-1:c);p.length=r?c-1:c;if(r){p.push(a.slice(c-1))}try{return n(this,...p)}catch(o){j(o)}}}"#,
        ),
        (
            r#"const C=[];for(let e=0;e<32;++e){const F=C.length===0?"":C.join(", ");const T=[...C,"body"].join(", ");t.teavmJso["createFunction"+e]=new Function("wrapCallFromJavaToJs",...C,"body",`return new Function('wrapCallFromJavaToJs', ${T}).bind(this, wrapCallFromJavaToJs);`).bind(null,k);t.teavmJso["bindFunction"+e]=(e,...t)=>e.bind(null,...t);t.teavmJso["callFunction"+e]=new Function("rethrowJsAsJava","fn",...C,`try {
    return fn(${F});
} catch (e) {
    rethrowJsAsJava(e);
}`).bind(null,x);t.teavmJso["callMethod"+e]=new Function("rethrowJsAsJava","getGlobalName","instance","method",...C,`try {
    return instance !== null
        ? instance[method](${F})
        : getGlobalName(method)(${F});
} catch (e) {
    rethrowJsAsJava(e);
}`).bind(null,x,D);t.teavmJso["construct"+e]=new Function("rethrowJsAsJava","constructor",...C,`try {
    return new constructor(${F});
} catch (e) {
    rethrowJsAsJava(e);
}`).bind(null,x);t.teavmJso["arrayOf"+e]=new Function(...C,"return ["+F+"]");C.push("p"+(e+1))}"#,
            r#"const C=/^(?:return )?globalThis\[Symbol\.for\('tachyon\.java\.host'\)\]\.(\w+)\(\w+\);?$/;const H=(b)=>{const m=C.exec(b.trim());if(m===null){throw new Error("Tachyon runs only its own TeaVM JavaScript bodies, because the page does not allow code made from text.")}return(v)=>globalThis[Symbol.for("tachyon.java.host")][m[1]](v)};for(let e=0;e<32;++e){t.teavmJso["createFunction"+e]=(...a)=>H(a[a.length-1]);t.teavmJso["bindFunction"+e]=(e,...t)=>e.bind(null,...t);t.teavmJso["callFunction"+e]=(f,...a)=>{try{return f(...a)}catch(r){x(r)}};t.teavmJso["callMethod"+e]=(i,m,...a)=>{try{return i!==null?i[m](...a):D(m)(...a)}catch(r){x(r)}};t.teavmJso["construct"+e]=(c,...a)=>{try{return new c(...a)}catch(r){x(r)}};t.teavmJso["arrayOf"+e]=(...a)=>a}"#,
        ),
    ];
    const MADE_FROM_TEXT: [&'static str; 2] = ["new Function", "eval("];

    pub(super) fn without_code_from_text(runtime: &str) -> Option<String> {
        let patched = Self::CODE_FROM_TEXT
            .iter()
            .try_fold(runtime.to_owned(), |text, (made, safe)| {
                text.contains(made).then(|| text.replacen(made, safe, 1))
            });
        patched.filter(|text| !Self::makes_code_from_text(text))
    }

    pub(super) fn identity() -> Vec<u8> {
        Self::CODE_FROM_TEXT
            .iter()
            .flat_map(|(_, safe)| safe.bytes())
            .collect()
    }

    fn makes_code_from_text(text: &str) -> bool {
        Self::MADE_FROM_TEXT.iter().any(|made| text.contains(made))
    }
}
